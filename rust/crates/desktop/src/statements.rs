//! The financial statements as plain rows, with no GPUI types, so they are unit-tested: which
//! report and parameters are chosen ([`ReportForm`], [`ReportParams`]), the statement laid out
//! as tables, key figures and footing checks ([`Statement`]), the CSV of each report
//! ([`csv_rows`]) and the display formats ([`format_minor`], [`format_date`]).
//!
//! Figures come straight from `ledger_core::reports`; this module only sets them out. The layout
//! and the CSV columns mirror the web screen, `ReportBody` and `reportCsv` in
//! `flowtrace/src/components/ledger/reports/statements.tsx`, with the row kinds of `statement.tsx`
//! (lines on a hairline, subtotals on a single rule, totals on a double rule). The grant report
//! has no web counterpart on the reports screen; it sets out `grantSummary`
//! (`flowtrace/src/lib/ledger/reports.ts:536-588`) the same way.

use std::fmt::{self, Write as _};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use ledger_core::Minor;
use ledger_core::csv::{CsvCell, to_csv};
use ledger_core::money::{format_major, js_round};
use ledger_core::reports::{self, Basis, LineFilter, ReportError, RestrictionTotals, WarningLevel};
use ledger_core::types::{AccountType, FundRestriction, Grant, GrantId, IsoDate, LedgerState};

/// One report of the picker, in picker order (`REPORTS` in `statements.tsx`, then the grant
/// report).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ReportKind {
    /// Trial balance as at a date.
    TrialBalance,
    /// Statement of financial position as at a date; the web screen's default.
    #[default]
    FinancialPosition,
    /// Statement of activities over a period, with the schedule by fund.
    Activities,
    /// Statement of functional expenses over a period.
    FunctionalExpenses,
    /// Statement of cash flows over a period, direct method.
    CashFlow,
    /// One grant's award, spend and burn as at a date.
    Grant,
}

/// Whether a report is as at one date or covers a period.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Span {
    /// As at the `to` date.
    Point,
    /// From `from` to `to`, both inclusive.
    Range,
}

impl ReportKind {
    /// Every report, in picker order.
    pub const ALL: [Self; 6] = [
        Self::TrialBalance,
        Self::FinancialPosition,
        Self::Activities,
        Self::FunctionalExpenses,
        Self::CashFlow,
        Self::Grant,
    ];

    /// The id the web screen uses in its URL and file names (`ReportId`).
    #[must_use]
    pub fn id(self) -> &'static str {
        match self {
            Self::TrialBalance => "trial-balance",
            Self::FinancialPosition => "position",
            Self::Activities => "activities",
            Self::FunctionalExpenses => "functional",
            Self::CashFlow => "cash-flow",
            Self::Grant => "grant",
        }
    }

    /// The report with this id.
    #[must_use]
    pub fn from_id(id: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|kind| kind.id() == id)
    }

    /// The picker label (`short`).
    #[must_use]
    pub fn short(self) -> &'static str {
        match self {
            Self::TrialBalance => "Trial balance",
            Self::FinancialPosition => "Financial position",
            Self::Activities => "Activities",
            Self::FunctionalExpenses => "Functional expenses",
            Self::CashFlow => "Cash flow",
            Self::Grant => "Grant report",
        }
    }

    /// The statement's title (`title`).
    #[must_use]
    pub fn title(self) -> &'static str {
        match self {
            Self::TrialBalance => "Trial balance",
            Self::FinancialPosition => "Statement of financial position",
            Self::Activities => "Statement of activities",
            Self::FunctionalExpenses => "Statement of functional expenses",
            Self::CashFlow => "Statement of cash flows",
            Self::Grant => "Grant report",
        }
    }

    /// As at a date or over a period (`span`).
    #[must_use]
    pub fn span(self) -> Span {
        match self {
            Self::TrialBalance | Self::FinancialPosition | Self::Grant => Span::Point,
            Self::Activities | Self::FunctionalExpenses | Self::CashFlow => Span::Range,
        }
    }

    /// Whether the report has an accrual and a cash view (`hasBasis`).
    #[must_use]
    pub fn has_basis(self) -> bool {
        !matches!(self, Self::CashFlow | Self::Grant)
    }
}

/// The parameters as the screen holds them: dates as typed, before validation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReportForm {
    /// The chosen report.
    pub kind: ReportKind,
    /// Start date text (`YYYY-MM-DD`); read by period reports only.
    pub from: String,
    /// End date or as-at date text (`YYYY-MM-DD`).
    pub to: String,
    /// Accrual or cash.
    pub basis: Basis,
    /// The grant of the grant report.
    pub grant: Option<GrantId>,
}

/// Valid parameters for one report.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReportParams {
    /// The report.
    pub kind: ReportKind,
    /// Start date; equal to `to` for reports as at a date.
    pub from: IsoDate,
    /// End date or as-at date.
    pub to: IsoDate,
    /// Accrual or cash; ignored by reports without a basis.
    pub basis: Basis,
    /// The grant, for the grant report.
    pub grant: Option<GrantId>,
}

/// Why a [`ReportForm`] cannot be run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParamError {
    /// The start date is not a calendar date in `YYYY-MM-DD`.
    From,
    /// The end or as-at date is not a calendar date in `YYYY-MM-DD`.
    To,
    /// The period starts after it ends.
    RangeReversed,
    /// The grant report has no grant chosen.
    NoGrant,
    /// The chosen grant is not in the books.
    UnknownGrant(GrantId),
}

impl fmt::Display for ParamError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::From => {
                f.write_str("Start date: enter a date as YYYY-MM-DD, for example 2026-01-31.")
            }
            Self::To => {
                f.write_str("End date: enter a date as YYYY-MM-DD, for example 2026-01-31.")
            }
            Self::RangeReversed => f.write_str(
                "The period starts after it ends. Choose a start date on or before the end date.",
            ),
            Self::NoGrant => f.write_str("Choose a grant."),
            Self::UnknownGrant(id) => write!(f, "Grant {id} is not in these books."),
        }
    }
}

impl ReportForm {
    /// The screen's opening parameters: the statement of financial position, year to date up to
    /// `as_of` (the web screen's `ytd` preset, `reports/index.tsx`), accrual basis and the first
    /// grant.
    #[must_use]
    pub fn new(state: &LedgerState, as_of: &IsoDate) -> Self {
        let from = presets(state, as_of)
            .into_iter()
            .find(|p| p.key == "ytd")
            .map_or_else(|| as_of.clone(), |p| p.from);
        Self {
            kind: ReportKind::default(),
            from: from.as_str().to_owned(),
            to: as_of.as_str().to_owned(),
            basis: Basis::Accrual,
            grant: state.grants.first().map(|g| g.id.clone()),
        }
    }

    /// Checks the dates and the grant against `state`. A report as at a date reads only `to`,
    /// as the web screen shows only that field for it.
    ///
    /// # Errors
    ///
    /// [`ParamError`] naming the first field that is wrong.
    pub fn validate(&self, state: &LedgerState) -> Result<ReportParams, ParamError> {
        let to = calendar_date(&self.to).ok_or(ParamError::To)?;
        let from = match self.kind.span() {
            Span::Point => to.clone(),
            Span::Range => calendar_date(&self.from).ok_or(ParamError::From)?,
        };
        if from > to {
            return Err(ParamError::RangeReversed);
        }
        if self.kind == ReportKind::Grant {
            let id = self.grant.as_ref().ok_or(ParamError::NoGrant)?;
            if !state.grants.iter().any(|g| &g.id == id) {
                return Err(ParamError::UnknownGrant(id.clone()));
            }
        }
        Ok(ReportParams {
            kind: self.kind,
            from,
            to,
            basis: self.basis,
            grant: self.grant.clone(),
        })
    }
}

/// `text` as a date when it is `YYYY-MM-DD` and on the calendar (the web screen's date field
/// refuses other days; the report functions would roll 30 February into March).
fn calendar_date(text: &str) -> Option<IsoDate> {
    let date = IsoDate::parse(text.trim()).ok()?;
    let (year, month, day) = parts(&date);
    ((1..=12).contains(&month) && (1..=days_in_month(year, month)).contains(&day)).then_some(date)
}

/// Year, month and day of a date whose shape [`IsoDate::parse`] checked.
fn parts(date: &IsoDate) -> (i64, i64, i64) {
    let s = date.as_str();
    let num = |range: std::ops::Range<usize>| s.get(range).and_then(|p| p.parse().ok());
    (
        num(0..4).unwrap_or(0),
        num(5..7).unwrap_or(0),
        num(8..10).unwrap_or(0),
    )
}

fn days_in_month(year: i64, month: i64) -> i64 {
    match month {
        2 if (year % 4 == 0 && year % 100 != 0) || year % 400 == 0 => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}

/// A period preset drawn from the books' own fiscal periods (`presets` in
/// `flowtrace/src/routes/_authenticated/reports/index.tsx`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Preset {
    /// `month`, `ytd` or `last`.
    pub key: &'static str,
    /// The button label.
    pub label: String,
    /// First day.
    pub from: IsoDate,
    /// Last day.
    pub to: IsoDate,
}

/// The period presets for `as_of`: this month (to `as_of` when it falls inside), year to date,
/// and the previous fiscal year when the books hold it (`presets` in `reports/index.tsx`).
#[must_use]
pub fn presets(state: &LedgerState, as_of: &IsoDate) -> Vec<Preset> {
    let mut periods: Vec<_> = state.periods.iter().collect();
    periods.sort_by(|a, b| a.start_date.cmp(&b.start_date));
    let Some(current) = periods
        .iter()
        .find(|p| &p.start_date <= as_of && as_of <= &p.end_date)
        .or_else(|| periods.last())
    else {
        return Vec::new();
    };
    let year: Vec<_> = periods
        .iter()
        .filter(|p| p.fiscal_year == current.fiscal_year)
        .collect();
    let last: Vec<_> = periods
        .iter()
        .filter(|p| p.fiscal_year == current.fiscal_year - 1)
        .collect();
    let mut out = vec![Preset {
        key: "month",
        label: format!("This month ({})", current.name),
        from: current.start_date.clone(),
        to: if as_of < &current.end_date {
            as_of.clone()
        } else {
            current.end_date.clone()
        },
    }];
    if let Some(first) = year.first() {
        out.push(Preset {
            key: "ytd",
            label: "Year to date".to_owned(),
            from: first.start_date.clone(),
            to: as_of.clone(),
        });
    }
    if let (Some(first), Some(end)) = (last.first(), last.last()) {
        out.push(Preset {
            key: "last",
            label: format!("Fiscal year {}", current.fiscal_year - 1),
            from: first.start_date.clone(),
            to: end.end_date.clone(),
        });
    }
    out
}

/// One cell of a statement row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Cell {
    /// An amount in minor units, shown with [`format_minor`].
    Money(Minor),
    /// Text as it stands, such as a percentage.
    Text(String),
}

impl Cell {
    /// The cell as shown on screen.
    #[must_use]
    pub fn display(&self) -> String {
        match self {
            Self::Money(m) => format_minor(*m),
            Self::Text(t) => t.clone(),
        }
    }
}

/// How a row is set (`RowKind` in `statement.tsx`, plus its group heading and gap rows).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RowKind {
    /// A group heading (Assets, Income): section-label type, no figures.
    Heading,
    /// A line of figures on a hairline.
    Line,
    /// A subtotal on a single ink rule, semibold.
    Subtotal,
    /// A grand total on a double rule, bold.
    Total,
    /// A spacer between groups.
    Gap,
}

/// One row of a statement table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row {
    /// How it is set.
    pub kind: RowKind,
    /// The account code, shown muted before the label.
    pub code: Option<String>,
    /// The label.
    pub label: String,
    /// A muted second line under the label.
    pub detail: Option<String>,
    /// One cell per column; empty for headings and gaps.
    pub cells: Vec<Cell>,
}

impl Row {
    fn new(kind: RowKind, label: impl Into<String>, cells: Vec<Cell>) -> Self {
        Self {
            kind,
            code: None,
            label: label.into(),
            detail: None,
            cells,
        }
    }

    fn line(label: impl Into<String>, cells: Vec<Cell>) -> Self {
        Self::new(RowKind::Line, label, cells)
    }

    fn account(code: &str, label: &str, cells: Vec<Cell>) -> Self {
        Self {
            code: Some(code.to_owned()),
            ..Self::line(label, cells)
        }
    }

    fn subtotal(label: impl Into<String>, cells: Vec<Cell>) -> Self {
        Self::new(RowKind::Subtotal, label, cells)
    }

    fn total(label: impl Into<String>, cells: Vec<Cell>) -> Self {
        Self::new(RowKind::Total, label, cells)
    }

    fn heading(label: &str) -> Self {
        Self::new(RowKind::Heading, label, Vec::new())
    }

    fn gap() -> Self {
        Self::new(RowKind::Gap, "", Vec::new())
    }

    fn detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = Some(detail.into());
        self
    }
}

/// Money cells.
fn money<const N: usize>(values: [Minor; N]) -> Vec<Cell> {
    values.into_iter().map(Cell::Money).collect()
}

/// A statement table (`StatementTable`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Table {
    /// A section label above the table, such as "Schedule by fund".
    pub title: Option<String>,
    /// The first column's head.
    pub first_column: String,
    /// The figure columns' heads.
    pub columns: Vec<String>,
    /// The rows.
    pub rows: Vec<Row>,
    /// Shown instead of the table when it has nothing to show (`Empty`).
    pub empty: Option<String>,
}

impl Table {
    fn new(first_column: &str, columns: &[&str], rows: Vec<Row>) -> Self {
        Self {
            title: None,
            first_column: first_column.to_owned(),
            columns: columns.iter().map(|&c| c.to_owned()).collect(),
            rows,
            empty: None,
        }
    }

    /// No rows, and `message` in their place, when `empty` holds.
    fn or_empty(mut self, empty: bool, message: String) -> Self {
        if empty {
            self.rows.clear();
            self.empty = Some(message);
        }
        self
    }
}

/// A footing check (`FootingCheck`): what must agree and by how much it does not.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Footing {
    /// What must agree.
    pub statement: String,
    /// The difference in minor units; 0 when it foots.
    pub difference: Minor,
    /// A muted note under the statement.
    pub detail: Option<String>,
}

impl Footing {
    fn new(statement: &str, difference: Minor, detail: Option<String>) -> Self {
        Self {
            statement: statement.to_owned(),
            difference,
            detail,
        }
    }

    /// Whether the statement foots.
    #[must_use]
    pub fn foots(&self) -> bool {
        self.difference == 0
    }
}

/// A key figure above a statement (`KeyFigure`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyFigure {
    /// The figure as shown.
    pub value: String,
    /// What it is.
    pub label: String,
}

impl KeyFigure {
    fn new(value: String, label: impl Into<String>) -> Self {
        Self {
            value,
            label: label.into(),
        }
    }
}

/// One block of a statement, in page order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Block {
    /// A row of key figures.
    Figures(Vec<KeyFigure>),
    /// A table.
    Table(Table),
    /// A footing check.
    Footing(Footing),
}

/// A statement ready to draw (`ReportBody` with its `StatementHead`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Statement {
    /// The report.
    pub kind: ReportKind,
    /// The title.
    pub title: String,
    /// "As at 4 Feb 2026" or "For the period 1 Jan 2026 to 4 Feb 2026".
    pub period: String,
    /// The currency the amounts are in.
    pub currency: String,
    /// "Accrual basis", "Cash basis" or "Direct method".
    pub basis: String,
    /// Figures, tables and footing checks, in page order.
    pub blocks: Vec<Block>,
}

impl Statement {
    /// The tables, in page order.
    pub fn tables(&self) -> impl Iterator<Item = &Table> {
        self.blocks.iter().filter_map(|b| match b {
            Block::Table(t) => Some(t),
            _ => None,
        })
    }

    /// The footing checks, in page order.
    pub fn footings(&self) -> impl Iterator<Item = &Footing> {
        self.blocks.iter().filter_map(|b| match b {
            Block::Footing(f) => Some(f),
            _ => None,
        })
    }
}

/// `BASIS_LABEL`, or what stands in its place for reports without a basis: the cash flow is
/// "Direct method" (`ReportBody`); the grant report reads accrual lines (`reports.ts:536-588`).
fn basis_label(params: &ReportParams) -> &'static str {
    match params.kind {
        ReportKind::CashFlow => "Direct method",
        ReportKind::Grant => "Accrual basis",
        _ => match params.basis {
            Basis::Accrual => "Accrual basis",
            Basis::Cash => "Cash basis",
        },
    }
}

/// `periodText`: "As at <to>" or "For the period <from> to <to>".
fn period_text(params: &ReportParams) -> String {
    match params.kind.span() {
        Span::Point => format!("As at {}", format_date(&params.to)),
        Span::Range => format!(
            "For the period {} to {}",
            format_date(&params.from),
            format_date(&params.to)
        ),
    }
}

/// The grant of a grant report.
fn grant_of<'s>(state: &'s LedgerState, params: &ReportParams) -> Result<&'s Grant, ReportError> {
    let id = params.grant.clone().unwrap_or_else(|| GrantId::from(""));
    state
        .grants
        .iter()
        .find(|g| g.id == id)
        .ok_or(ReportError::UnknownGrant(id))
}

/// The statement for `params` over `state`, laid out as `ReportBody` lays it out.
///
/// # Errors
///
/// [`ReportError`] where the report function refuses the parameters: a date off the calendar,
/// or a grant not in the books.
pub fn statement(state: &LedgerState, params: &ReportParams) -> Result<Statement, ReportError> {
    let mut currency = state.settings.base_currency.clone();
    let mut title = params.kind.title().to_owned();
    let blocks = match params.kind {
        ReportKind::TrialBalance => trial_balance_blocks(state, params),
        ReportKind::FinancialPosition => position_blocks(state, params),
        ReportKind::Activities => activities_blocks(state, params)?,
        ReportKind::FunctionalExpenses => functional_blocks(state, params, &currency),
        ReportKind::CashFlow => cash_flow_blocks(state, params)?,
        ReportKind::Grant => {
            let grant = grant_of(state, params)?;
            title = format!("Grant report: {}", grant.name);
            currency.clone_from(&grant.currency);
            grant_blocks(state, grant, params)?
        }
    };
    Ok(Statement {
        kind: params.kind,
        title,
        period: period_text(params),
        currency,
        basis: basis_label(params).to_owned(),
        blocks,
    })
}

/// `TrialBalance` in `statements.tsx`.
fn trial_balance_blocks(state: &LedgerState, p: &ReportParams) -> Vec<Block> {
    let tb = reports::trial_balance(state, Some(&p.to), p.basis);
    let mut rows: Vec<Row> = tb
        .rows
        .iter()
        .map(|r| Row::account(&r.account.code, &r.account.name, money([r.debit, r.credit])))
        .collect();
    rows.push(Row::total("Totals", money([tb.debit, tb.credit])));
    let table = Table::new("Account", &["Debit", "Credit"], rows).or_empty(
        tb.rows.is_empty(),
        format!("No posted balances on or before {}.", format_date(&p.to)),
    );
    vec![
        Block::Table(table),
        Block::Footing(Footing::new(
            "Total debits equal total credits.",
            tb.difference,
            None,
        )),
    ]
}

/// `Position` in `statements.tsx`.
fn position_blocks(state: &LedgerState, p: &ReportParams) -> Vec<Block> {
    let fp = reports::financial_position(state, &p.to, p.basis);
    let lines = |rows: &[reports::StatementRow]| -> Vec<Row> {
        rows.iter()
            .map(|r| Row::account(&r.account.code, &r.account.name, money([r.amount])))
            .collect()
    };
    let mut rows = vec![Row::heading("Assets")];
    rows.extend(lines(&fp.assets));
    rows.push(Row::subtotal("Total assets", money([fp.total_assets])));
    rows.push(Row::gap());
    rows.push(Row::heading("Liabilities"));
    if fp.liabilities.is_empty() {
        rows.push(Row::line("No liabilities", money([0])));
    }
    rows.extend(lines(&fp.liabilities));
    rows.push(Row::subtotal(
        "Total liabilities",
        money([fp.total_liabilities]),
    ));
    rows.push(Row::gap());
    rows.push(Row::heading("Net assets"));
    rows.push(Row::line(
        "Without donor restrictions",
        money([fp.net_assets.unrestricted]),
    ));
    rows.push(Row::line(
        "With donor restrictions",
        money([fp.net_assets.restricted]),
    ));
    rows.push(Row::subtotal(
        "Total net assets",
        money([fp.total_net_assets]),
    ));
    rows.push(Row::gap());
    rows.push(Row::total(
        "Total liabilities and net assets",
        money([fp.total_liabilities + fp.total_net_assets]),
    ));
    let date = format_date(&p.to);
    vec![
        Block::Table(Table::new("", &[date.as_str()], rows)),
        Block::Footing(Footing::new(
            "Assets equal liabilities plus net assets.",
            fp.difference,
            Some(
                "Net assets include this year's surplus or deficit to date, split by fund \
                 restriction."
                    .to_owned(),
            ),
        )),
    ]
}

fn restriction_cells(x: &RestrictionTotals) -> Vec<Cell> {
    money([x.unrestricted, x.restricted, x.total])
}

/// "Without donor restrictions" or "With donor restrictions" for a fund.
fn restriction_label(r: FundRestriction) -> &'static str {
    match r {
        FundRestriction::Unrestricted => "Without donor restrictions",
        FundRestriction::TemporarilyRestricted | FundRestriction::PermanentlyRestricted => {
            "With donor restrictions"
        }
    }
}

/// A fund restriction as the books write it (`types.ts`), for the CSV.
fn restriction_id(r: FundRestriction) -> &'static str {
    match r {
        FundRestriction::Unrestricted => "unrestricted",
        FundRestriction::TemporarilyRestricted => "temporarily_restricted",
        FundRestriction::PermanentlyRestricted => "permanently_restricted",
    }
}

/// The statement of activities' footing check (`Activities` in `statements.tsx`): the change
/// in net assets over the period, plus what was posted straight to net assets (opening
/// balances), equals the movement in net assets on the balance sheet between the day before
/// `from` and `to`.
fn net_assets_check(
    state: &LedgerState,
    p: &ReportParams,
    change: Minor,
) -> Result<Footing, ReportError> {
    let day_before = reports::day_before(&p.from)?;
    let before = reports::financial_position(state, &day_before, p.basis);
    let after = reports::financial_position(state, &p.to, p.basis);
    let movement = after.total_net_assets - before.total_net_assets;
    let filter = LineFilter {
        from: Some(p.from.clone()),
        to: Some(p.to.clone()),
        basis: p.basis,
        exclude_closing: true,
        ..LineFilter::default()
    };
    let equity: Minor = reports::posted_lines(state, &filter)
        .iter()
        .filter(|l| {
            state
                .accounts
                .iter()
                .rev()
                .find(|a| a.id == l.line.account_id)
                .is_some_and(|a| a.account_type == AccountType::Equity)
        })
        .map(|l| l.line.base_amount)
        .sum();
    let direct = -equity;

    let mut detail = format!(
        "Net assets {} on {}, {} on {}.",
        format_compact(before.total_net_assets),
        format_date(&day_before),
        format_compact(after.total_net_assets),
        format_date(&p.to)
    );
    if direct != 0 {
        let _ = write!(
            detail,
            " Includes {} posted straight to net assets, such as opening balances.",
            format_compact(direct)
        );
    }
    detail.push_str(" Year-end closing entries are left out.");
    Ok(Footing::new(
        "Change in net assets equals the movement in net assets on the statement of financial \
         position.",
        change + direct - movement,
        Some(detail),
    ))
}

/// `Activities` in `statements.tsx`: the statement, its footing check and the schedule by fund.
fn activities_blocks(state: &LedgerState, p: &ReportParams) -> Result<Vec<Block>, ReportError> {
    let a = reports::activities(state, &p.from, &p.to, p.basis);

    let mut rows = vec![Row::heading("Income")];
    rows.extend(a.income.iter().map(|r| {
        Row::account(
            &r.account.code,
            &r.account.name,
            money([r.unrestricted, r.restricted, r.total]),
        )
    }));
    rows.push(Row::subtotal(
        "Total income",
        restriction_cells(&a.total_income),
    ));
    rows.push(Row::gap());
    rows.push(Row::heading("Expenses"));
    rows.extend(a.expenses.iter().map(|r| {
        Row::account(
            &r.account.code,
            &r.account.name,
            money([r.unrestricted, r.restricted, r.total]),
        )
    }));
    rows.push(Row::subtotal(
        "Total expenses",
        restriction_cells(&a.total_expenses),
    ));
    rows.push(Row::gap());
    rows.push(Row::total(
        "Change in net assets",
        restriction_cells(&a.change_in_net_assets),
    ));
    let main = Table::new(
        "",
        &["Without restrictions", "With restrictions", "Total"],
        rows,
    )
    .or_empty(
        a.income.is_empty() && a.expenses.is_empty(),
        "No income or expenses posted in this period.".to_owned(),
    );

    let footing = net_assets_check(state, p, a.change_in_net_assets.total)?;

    let mut funds: Vec<Row> = a
        .by_fund
        .iter()
        .map(|f| {
            Row::line(
                f.fund.name.as_str(),
                money([f.income, f.expenses, f.change]),
            )
            .detail(restriction_label(f.fund.restriction))
        })
        .collect();
    funds.push(Row::total(
        "All funds",
        money([
            a.total_income.total,
            a.total_expenses.total,
            a.change_in_net_assets.total,
        ]),
    ));
    let mut schedule = Table::new("Fund", &["Income", "Expenses", "Change"], funds).or_empty(
        a.by_fund.is_empty(),
        "No fund had income or expenses in this period.".to_owned(),
    );
    schedule.title = Some("Schedule by fund".to_owned());

    Ok(vec![
        Block::Table(main),
        Block::Footing(footing),
        Block::Table(schedule),
    ])
}

/// `Math.round(x)` as a whole number for display.
#[expect(
    clippy::cast_possible_truncation,
    reason = "percentages and ratios of safe amounts, far inside i64"
)]
fn rounded(x: f64) -> i64 {
    js_round(x) as i64
}

/// `Functional` in `statements.tsx`.
fn functional_blocks(state: &LedgerState, p: &ReportParams, currency: &str) -> Vec<Block> {
    let fe = reports::functional_expenses(state, &p.from, &p.to, p.basis);
    let act = reports::activities(state, &p.from, &p.to, p.basis);
    let t = &fe.totals;
    let pct = |n: Minor| {
        if t.total > 0 {
            format!("{}%", rounded(js(n) / js(t.total) * 100.0))
        } else {
            "\u{2013}".to_owned()
        }
    };
    let ratio = if t.total > 0 {
        format!("{}%", rounded(fe.program_ratio * 100.0))
    } else {
        "\u{2013}".to_owned()
    };
    let figures = vec![
        KeyFigure::new(ratio, "Programme ratio"),
        KeyFigure::new(format_compact(t.program), format!("Programmes, {currency}")),
        KeyFigure::new(format_compact(t.management), "Management and general"),
        KeyFigure::new(format_compact(t.fundraising), "Fundraising"),
    ];
    let mut rows: Vec<Row> = fe
        .rows
        .iter()
        .map(|r| {
            Row::account(
                &r.account.code,
                &r.account.name,
                money([r.program, r.management, r.fundraising, r.total]),
            )
        })
        .collect();
    rows.push(Row::total(
        "Total expenses",
        money([t.program, t.management, t.fundraising, t.total]),
    ));
    rows.push(Row::line(
        "Share of total",
        [t.program, t.management, t.fundraising, t.total]
            .into_iter()
            .map(|n| Cell::Text(pct(n)))
            .collect(),
    ));
    let table = Table::new(
        "Expense",
        &["Programmes", "Management", "Fundraising", "Total"],
        rows,
    )
    .or_empty(
        fe.rows.is_empty(),
        "No expenses posted in this period.".to_owned(),
    );
    vec![
        Block::Figures(figures),
        Block::Table(table),
        Block::Footing(Footing::new(
            "Total expenses equal total expenses on the statement of activities.",
            t.total - act.total_expenses.total,
            Some(
                "Each expense account carries one function; accounts without one count as \
                 programmes."
                    .to_owned(),
            ),
        )),
    ]
}

/// `CashFlowStatement` in `statements.tsx`.
fn cash_flow_blocks(state: &LedgerState, p: &ReportParams) -> Result<Vec<Block>, ReportError> {
    let cf = reports::cash_flow(state, &p.from, &p.to)?;
    let day_before = reports::day_before(&p.from)?;
    let rows = vec![
        Row::heading("Operating activities"),
        Row::line(
            "Cash received from donors and other income",
            money([cf.operating.receipts]),
        ),
        Row::line(
            "Cash paid to suppliers, staff and programmes",
            money([cf.operating.payments]),
        ),
        Row::subtotal(
            "Net cash from operating activities",
            money([cf.operating.net]),
        ),
        Row::gap(),
        Row::heading("Investing activities"),
        Row::subtotal("Net cash from investing activities", money([cf.investing])),
        Row::gap(),
        Row::heading("Financing activities"),
        Row::subtotal("Net cash from financing activities", money([cf.financing])),
        Row::gap(),
        Row::subtotal("Net change in cash", money([cf.net])),
        Row::line(
            format!("Cash at {}", format_date(&day_before)),
            money([cf.opening]),
        ),
        Row::total(
            format!("Cash at {}", format_date(&p.to)),
            money([cf.closing]),
        ),
    ];
    Ok(vec![
        Block::Table(Table::new("", &["Amount"], rows)),
        Block::Footing(Footing::new(
            "Opening cash plus the net change equals closing cash.",
            cf.difference,
            Some(
                "Transfers between the organisation's own bank and mobile-money accounts are not \
                 cash flows and are left out."
                    .to_owned(),
            ),
        )),
    ])
}

/// The words for a grant's `warningLevel` (`reports.ts:466-470`).
fn warning_label(level: WarningLevel) -> &'static str {
    match level {
        WarningLevel::Within => "Within the award",
        WarningLevel::Warning => "80% or more of the award used",
        WarningLevel::Over => "Over the award",
    }
}

/// A grant's `warningLevel` as TS writes it.
fn warning_id(level: WarningLevel) -> &'static str {
    match level {
        WarningLevel::Within => "within",
        WarningLevel::Warning => "warning",
        WarningLevel::Over => "over",
    }
}

/// The grant report: `grantSummary` (`reports.ts:536-588`) set out as a statement.
fn grant_blocks(
    state: &LedgerState,
    grant: &Grant,
    p: &ReportParams,
) -> Result<Vec<Block>, ReportError> {
    let g = reports::grant_summary(state, &grant.id, &p.to)?;
    let months = |m: f64| to_fixed(m, 1);
    let figures = vec![
        KeyFigure::new(
            format!("{}%", rounded(g.used * 100.0)),
            "Of the award spent or committed",
        ),
        KeyFigure::new(
            format_compact(g.remaining),
            format!("Remaining, {}", grant.currency),
        ),
        KeyFigure::new(format_compact(g.burn_per_month), "Spent per month"),
    ];
    let rows = vec![
        Row::line("Award", money([g.award])).detail(format!(
            "{} to {}, from {}",
            format_date(&grant.start_date),
            format_date(&grant.end_date),
            grant.donor_name
        )),
        Row::line("Received", money([g.received])).detail("Income received on the grant"),
        Row::line("Spent", money([g.spent])).detail("Expenses posted to the grant"),
        Row::line("Committed", money([g.committed]))
            .detail("Approved bills not yet posted and submitted claims"),
        Row::total("Remaining", money([g.remaining])),
        Row::gap(),
        Row::heading("Burn"),
        Row::line("Burn per month", money([g.burn_per_month])),
        Row::line(
            "Months of award left at this burn",
            vec![Cell::Text(
                g.months_left.map_or_else(|| "\u{2013}".to_owned(), months),
            )],
        ),
        Row::line(
            "Months to the grant's end",
            vec![Cell::Text(months(g.months_to_end))],
        ),
        Row::line(
            "Status",
            vec![Cell::Text(warning_label(g.status).to_owned())],
        ),
    ];
    Ok(vec![
        Block::Figures(figures),
        Block::Table(Table::new("", &["Amount"], rows)),
        Block::Footing(Footing::new(
            "The award less spent and committed equals what remains.",
            g.award - g.spent - g.committed - g.remaining,
            None,
        )),
    ])
}

/// `csvAmount` (`flowtrace/src/components/ledger/export.ts`): `(minor / 100).toFixed(2)`.
#[must_use]
pub fn csv_amount(minor: Minor) -> CsvCell {
    CsvCell::Text(format_major(minor))
}

fn text(s: impl Into<String>) -> CsvCell {
    CsvCell::Text(s.into())
}

/// The report as spreadsheet rows, amounts in major units (`reportCsv` in `statements.tsx`):
/// a title block of four rows, then one row per table row with a section column where the
/// statement has sections. The grant report follows the same pattern.
///
/// # Errors
///
/// [`ReportError`] where the report function refuses the parameters.
pub fn csv_rows(state: &LedgerState, p: &ReportParams) -> Result<Vec<Vec<CsvCell>>, ReportError> {
    let currency = match p.kind {
        ReportKind::Grant => grant_of(state, p)?.currency.clone(),
        _ => state.settings.base_currency.clone(),
    };
    let mut rows = vec![
        vec![text(p.kind.title())],
        vec![text(period_text(p))],
        vec![text(format!("Amounts in {currency}")), text(basis_label(p))],
        vec![],
    ];
    match p.kind {
        ReportKind::TrialBalance => csv_trial_balance(state, p, &mut rows),
        ReportKind::FinancialPosition => csv_position(state, p, &mut rows),
        ReportKind::Activities => csv_activities(state, p, &mut rows),
        ReportKind::FunctionalExpenses => csv_functional(state, p, &mut rows),
        ReportKind::CashFlow => csv_cash_flow(state, p, &mut rows)?,
        ReportKind::Grant => csv_grant(state, p, &mut rows)?,
    }
    Ok(rows)
}

/// The `trial_balance` rows of [`csv_rows`] after the title block.
fn csv_trial_balance(state: &LedgerState, p: &ReportParams, rows: &mut Vec<Vec<CsvCell>>) {
    let c = csv_amount;
    let tb = reports::trial_balance(state, Some(&p.to), p.basis);
    rows.push(vec![
        text("Code"),
        text("Account"),
        text("Debit"),
        text("Credit"),
    ]);
    rows.extend(tb.rows.iter().map(|r| {
        vec![
            text(r.account.code.as_str()),
            text(r.account.name.as_str()),
            c(r.debit),
            c(r.credit),
        ]
    }));
    rows.push(vec![text(""), text("Totals"), c(tb.debit), c(tb.credit)]);
    rows.push(vec![text(""), text("Difference"), c(tb.difference)]);
}

/// The `position` rows of [`csv_rows`] after the title block.
fn csv_position(state: &LedgerState, p: &ReportParams, rows: &mut Vec<Vec<CsvCell>>) {
    let c = csv_amount;
    let fp = reports::financial_position(state, &p.to, p.basis);
    let line = |s: &str, code: &str, l: &str, m: Minor| vec![text(s), text(code), text(l), c(m)];
    rows.push(vec![
        text("Section"),
        text("Code"),
        text("Line"),
        text("Amount"),
    ]);
    rows.extend(
        fp.assets
            .iter()
            .map(|r| line("Assets", &r.account.code, &r.account.name, r.amount)),
    );
    rows.push(line("Assets", "", "Total assets", fp.total_assets));
    rows.extend(
        fp.liabilities
            .iter()
            .map(|r| line("Liabilities", &r.account.code, &r.account.name, r.amount)),
    );
    rows.push(line(
        "Liabilities",
        "",
        "Total liabilities",
        fp.total_liabilities,
    ));
    rows.push(line(
        "Net assets",
        "",
        "Without donor restrictions",
        fp.net_assets.unrestricted,
    ));
    rows.push(line(
        "Net assets",
        "",
        "With donor restrictions",
        fp.net_assets.restricted,
    ));
    rows.push(line(
        "Net assets",
        "",
        "Total net assets",
        fp.total_net_assets,
    ));
    rows.push(line(
        "",
        "",
        "Total liabilities and net assets",
        fp.total_liabilities + fp.total_net_assets,
    ));
    rows.push(line("", "", "Difference", fp.difference));
}

/// The `activities` rows of [`csv_rows`] after the title block.
fn csv_activities(state: &LedgerState, p: &ReportParams, rows: &mut Vec<Vec<CsvCell>>) {
    let c = csv_amount;
    let a = reports::activities(state, &p.from, &p.to, p.basis);
    let row = |s: &str, code: &str, l: &str, u: Minor, r: Minor, t: Minor| {
        vec![text(s), text(code), text(l), c(u), c(r), c(t)]
    };
    let total = |s: &str, l: &str, x: &RestrictionTotals| {
        row(s, "", l, x.unrestricted, x.restricted, x.total)
    };
    rows.push(
        [
            "Section",
            "Code",
            "Line",
            "Without restrictions",
            "With restrictions",
            "Total",
        ]
        .map(text)
        .to_vec(),
    );
    rows.extend(a.income.iter().map(|r| {
        row(
            "Income",
            &r.account.code,
            &r.account.name,
            r.unrestricted,
            r.restricted,
            r.total,
        )
    }));
    rows.push(total("Income", "Total income", &a.total_income));
    rows.extend(a.expenses.iter().map(|r| {
        row(
            "Expenses",
            &r.account.code,
            &r.account.name,
            r.unrestricted,
            r.restricted,
            r.total,
        )
    }));
    rows.push(total("Expenses", "Total expenses", &a.total_expenses));
    rows.push(total("", "Change in net assets", &a.change_in_net_assets));
    rows.push(vec![]);
    rows.push(vec![text("Schedule by fund")]);
    rows.push(
        ["Fund", "Restriction", "Income", "Expenses", "Change"]
            .map(text)
            .to_vec(),
    );
    rows.extend(a.by_fund.iter().map(|f| {
        vec![
            text(f.fund.name.as_str()),
            text(restriction_id(f.fund.restriction)),
            c(f.income),
            c(f.expenses),
            c(f.change),
        ]
    }));
}

/// The `functional` rows of [`csv_rows`] after the title block.
fn csv_functional(state: &LedgerState, p: &ReportParams, rows: &mut Vec<Vec<CsvCell>>) {
    let c = csv_amount;
    let fe = reports::functional_expenses(state, &p.from, &p.to, p.basis);
    let t = &fe.totals;
    rows.push(
        [
            "Code",
            "Expense",
            "Programmes",
            "Management",
            "Fundraising",
            "Total",
        ]
        .map(text)
        .to_vec(),
    );
    rows.extend(fe.rows.iter().map(|r| {
        vec![
            text(r.account.code.as_str()),
            text(r.account.name.as_str()),
            c(r.program),
            c(r.management),
            c(r.fundraising),
            c(r.total),
        ]
    }));
    rows.push(vec![
        text(""),
        text("Total expenses"),
        c(t.program),
        c(t.management),
        c(t.fundraising),
        c(t.total),
    ]);
    rows.push(vec![
        text(""),
        text("Programme ratio"),
        text(to_fixed(fe.program_ratio, 4)),
    ]);
}

/// The `cash_flow` rows of [`csv_rows`] after the title block.
fn csv_cash_flow(
    state: &LedgerState,
    p: &ReportParams,
    rows: &mut Vec<Vec<CsvCell>>,
) -> Result<(), ReportError> {
    let c = csv_amount;
    let cf = reports::cash_flow(state, &p.from, &p.to)?;
    let line = |s: &str, l: &str, m: Minor| vec![text(s), text(l), c(m)];
    rows.push(["Section", "Line", "Amount"].map(text).to_vec());
    rows.push(line("Operating", "Cash received", cf.operating.receipts));
    rows.push(line("Operating", "Cash paid", cf.operating.payments));
    rows.push(line(
        "Operating",
        "Net cash from operating activities",
        cf.operating.net,
    ));
    rows.push(line(
        "Investing",
        "Net cash from investing activities",
        cf.investing,
    ));
    rows.push(line(
        "Financing",
        "Net cash from financing activities",
        cf.financing,
    ));
    rows.push(line("", "Net change in cash", cf.net));
    rows.push(line("", "Opening cash", cf.opening));
    rows.push(line("", "Closing cash", cf.closing));
    rows.push(line("", "Difference", cf.difference));
    Ok(())
}

/// The `grant` rows of [`csv_rows`] after the title block.
fn csv_grant(
    state: &LedgerState,
    p: &ReportParams,
    rows: &mut Vec<Vec<CsvCell>>,
) -> Result<(), ReportError> {
    let c = csv_amount;
    let grant = grant_of(state, p)?;
    let g = reports::grant_summary(state, &grant.id, &p.to)?;
    rows.push(vec![
        text("Grant"),
        text(grant.name.as_str()),
        text(grant.id.as_str()),
    ]);
    rows.push(vec![text("Line"), text("Amount")]);
    rows.push(vec![text("Award"), c(g.award)]);
    rows.push(vec![text("Received"), c(g.received)]);
    rows.push(vec![text("Spent"), c(g.spent)]);
    rows.push(vec![text("Committed"), c(g.committed)]);
    rows.push(vec![text("Remaining"), c(g.remaining)]);
    rows.push(vec![text("Used"), text(to_fixed(g.used, 4))]);
    rows.push(vec![text("Burn per month"), c(g.burn_per_month)]);
    rows.push(vec![
        text("Months left"),
        g.months_left
            .map_or(CsvCell::Empty, |m| text(to_fixed(m, 2))),
    ]);
    rows.push(vec![
        text("Months to end"),
        text(to_fixed(g.months_to_end, 2)),
    ]);
    rows.push(vec![text("Status"), text(warning_id(g.status))]);
    Ok(())
}

/// The report's CSV text through `ledger_core::csv::to_csv` (`downloadCsv` writes `toCsv`).
///
/// # Errors
///
/// [`ReportError`] where the report function refuses the parameters.
pub fn export_csv(state: &LedgerState, params: &ReportParams) -> Result<String, ReportError> {
    Ok(to_csv(&csv_rows(state, params)?))
}

/// Why an export did not write its file.
#[derive(Debug, thiserror::Error)]
pub enum ExportError {
    /// The report refused its parameters.
    #[error(transparent)]
    Report(#[from] ReportError),
    /// The file could not be written.
    #[error("Could not write {path}: {source}")]
    Write {
        /// Where.
        path: PathBuf,
        /// Why.
        source: std::io::Error,
    },
}

/// Writes the report's CSV to `path`, replacing a file already there.
///
/// # Errors
///
/// [`ExportError`] when the report refuses its parameters or the file cannot be written.
pub fn write_export(
    state: &LedgerState,
    params: &ReportParams,
    path: &Path,
) -> Result<(), ExportError> {
    let csv = export_csv(state, params)?;
    std::fs::write(path, csv).map_err(|source| ExportError::Write {
        path: path.to_owned(),
        source,
    })
}

/// The export's file name without extension, as the web screen names its download
/// (`exportCsv` in `reports/index.tsx`): `<id>_<date or from_to_to>[_<basis>]`; the grant
/// report puts the grant id after its own id.
#[must_use]
pub fn file_stem(params: &ReportParams) -> String {
    let range = match params.kind.span() {
        Span::Point => params.to.as_str().to_owned(),
        Span::Range => format!("{}_to_{}", params.from, params.to),
    };
    let id = match (&params.kind, &params.grant) {
        (ReportKind::Grant, Some(grant)) => format!("grant_{grant}"),
        (kind, _) => kind.id().to_owned(),
    };
    let basis = if params.kind.has_basis() {
        match params.basis {
            Basis::Accrual => "_accrual",
            Basis::Cash => "_cash",
        }
    } else {
        ""
    };
    format!("{id}_{range}{basis}")
}

/// `<home>/Downloads/<file_stem>.csv`, or the bare file name when there is no home.
#[must_use]
pub fn default_export_path(home: Option<&Path>, params: &ReportParams) -> PathBuf {
    let name = format!("{}.csv", file_stem(params));
    home.map_or_else(
        || PathBuf::from(&name),
        |home| home.join("Downloads").join(&name),
    )
}

/// An amount for display (`formatMinor` in `flowtrace/src/components/ledger/money.tsx`): two
/// decimals with `en-KE` thousands separators, negatives in parentheses, zero as an en dash.
/// Worked in integers, so no amount rounds.
#[must_use]
pub fn format_minor(minor: Minor) -> String {
    if minor == 0 {
        return "\u{2013}".to_owned();
    }
    let abs = minor.unsigned_abs();
    let text = format!("{}.{:02}", group(abs / 100), abs % 100);
    if minor < 0 { format!("({text})") } else { text }
}

/// A whole number with commas every three digits.
fn group(n: u64) -> String {
    let digits = n.to_string();
    let mut out = String::with_capacity(digits.len() + digits.len() / 3);
    for (i, ch) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(ch);
    }
    out
}

/// A compact figure for headline numbers (`formatMinorCompact` in `money.tsx`): 15.7M, 850K,
/// 9,500, with a minus sign (U+2212) for negatives.
#[must_use]
pub fn format_compact(minor: Minor) -> String {
    let major = js(minor) / 100.0;
    let a = major.abs();
    let sign = if major < 0.0 { "\u{2212}" } else { "" };
    if a >= 1_000_000.0 {
        let m = to_fixed(a / 1_000_000.0, 1);
        let m = m.strip_suffix(".0").unwrap_or(&m);
        format!("{sign}{m}M")
    } else if a >= 10_000.0 {
        format!("{sign}{}K", rounded(a / 1_000.0))
    } else {
        format!("{sign}{}", group(rounded(a).unsigned_abs()))
    }
}

/// `en-GB` short month names as current ICU writes them (September is "Sept").
const MONTHS: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sept", "Oct", "Nov", "Dec",
];

/// A date as the web app prints it (`formatDate` in `flowtrace/src/lib/format.ts`, `en-GB`
/// day, short month, year, in UTC): `4 Feb 2026`.
#[must_use]
pub fn format_date(date: &IsoDate) -> String {
    let (year, month, day) = parts(date);
    let name = usize::try_from(month - 1)
        .ok()
        .and_then(|m| MONTHS.get(m))
        .copied()
        .unwrap_or("");
    format!("{day} {name} {year}")
}

/// `Number.prototype.toFixed(digits)` for finite numbers below 1e21: rounded from the exact
/// binary value, ties away from zero (ECMA-262: "if there are two such n, pick the larger n"),
/// as `ledger_core::money::format_major` does for two places.
#[must_use]
pub fn to_fixed(x: f64, digits: usize) -> String {
    // Rust prints the exact decimal expansion of a double at any precision; 1100 places hold
    // every one.
    let exact = format!("{:.1100}", x.abs());
    let (whole, frac) = exact.split_once('.').unwrap_or((exact.as_str(), ""));
    let mut kept: Vec<u8> = whole.bytes().chain(frac.bytes().take(digits)).collect();
    if frac.as_bytes().get(digits).is_some_and(|&b| b >= b'5') {
        let mut i = kept.len();
        loop {
            if i == 0 {
                kept.insert(0, b'1');
                break;
            }
            i -= 1;
            if kept[i] == b'9' {
                kept[i] = b'0';
            } else {
                kept[i] += 1;
                break;
            }
        }
    }
    let split = kept.len() - digits;
    let (int, dec) = kept.split_at(split);
    let mut out = String::new();
    if x < 0.0 {
        out.push('-');
    }
    out.push_str(&String::from_utf8_lossy(int));
    if digits > 0 {
        out.push('.');
        out.push_str(&String::from_utf8_lossy(dec));
    }
    out
}

/// A safe amount as the double TS holds.
#[expect(clippy::cast_precision_loss, reason = "a JS number, as TS holds it")]
fn js(n: Minor) -> f64 {
    n as f64
}

/// The calendar date `days` after 1970-01-01 (Howard Hinnant's `civil_from_days`,
/// <https://howardhinnant.github.io/date_algorithms.html>), years kept in 0000-9999.
#[must_use]
pub fn date_from_days(days: i64) -> IsoDate {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = (yoe + era * 400 + i64::from(month <= 2)).clamp(0, 9999);
    IsoDate::parse(&format!("{year:04}-{month:02}-{day:02}"))
        .unwrap_or_else(|_| unreachable!("four, two and two digits make an ISO date"))
}

/// Today in UTC, the as-at date for books that are not the demo's (`useLedgerAsOf` outside dev
/// mode, `flowtrace/src/lib/ledger-client.ts`).
#[must_use]
pub fn today_utc() -> IsoDate {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    date_from_days(i64::try_from(secs / 86_400).unwrap_or(0))
}

#[cfg(test)]
mod tests {
    #![expect(clippy::unwrap_used, reason = "tests may unwrap")]

    use super::*;
    use ledger_core::demo::{DEMO_AS_OF, build_demo_ledger};
    use ledger_core::env::fixed;
    use ledger_core::reports;
    use ledger_core::setup::create_org_ledger;

    fn d(s: &str) -> IsoDate {
        IsoDate::parse(s).unwrap()
    }

    fn demo() -> LedgerState {
        build_demo_ledger().unwrap()
    }

    fn empty_books() -> LedgerState {
        create_org_ledger("org-empty", "KES", 2026, &mut fixed::test()).unwrap()
    }

    fn params(kind: ReportKind, from: &str, to: &str, basis: Basis) -> ReportParams {
        ReportParams {
            kind,
            from: d(from),
            to: d(to),
            basis,
            grant: None,
        }
    }

    fn labels(table: &Table, kind: RowKind) -> Vec<&str> {
        table
            .rows
            .iter()
            .filter(|r| r.kind == kind)
            .map(|r| r.label.as_str())
            .collect()
    }

    #[test]
    fn report_kinds_round_trip_their_ids() {
        for kind in ReportKind::ALL {
            assert_eq!(ReportKind::from_id(kind.id()), Some(kind));
        }
        assert_eq!(ReportKind::from_id("budget"), None);
        assert_eq!(ReportKind::TrialBalance.span(), Span::Point);
        assert_eq!(ReportKind::Activities.span(), Span::Range);
        assert!(!ReportKind::CashFlow.has_basis());
        assert!(ReportKind::FunctionalExpenses.has_basis());
    }

    #[test]
    fn the_form_opens_on_the_position_year_to_date() {
        let books = demo();
        let form = ReportForm::new(&books, &d(DEMO_AS_OF));
        assert_eq!(form.kind, ReportKind::FinancialPosition);
        assert_eq!(form.from, "2026-01-01");
        assert_eq!(form.to, "2026-02-04");
        assert_eq!(form.basis, Basis::Accrual);
        assert_eq!(form.grant, Some(GrantId::from("grant-harambee-2026")));
    }

    #[test]
    fn validation_names_the_wrong_field() {
        let books = demo();
        let mut form = ReportForm::new(&books, &d(DEMO_AS_OF));
        form.kind = ReportKind::Activities;
        let ok = form.validate(&books).unwrap();
        assert_eq!(
            (ok.from.as_str(), ok.to.as_str()),
            ("2026-01-01", "2026-02-04")
        );

        for bad in [
            "04/02/2026",
            "2026-13-01",
            "2025-02-29",
            "2026-04-31",
            "",
            "2026-1-01",
        ] {
            let mut f = form.clone();
            f.to = bad.to_owned();
            assert_eq!(f.validate(&books), Err(ParamError::To), "{bad:?}");
            let mut f = form.clone();
            f.from = bad.to_owned();
            assert_eq!(f.validate(&books), Err(ParamError::From), "{bad:?}");
        }
        let mut leap = form.clone();
        leap.from = "2024-02-29".to_owned();
        assert!(leap.validate(&books).is_ok());

        let mut reversed = form.clone();
        reversed.from = "2026-03-01".to_owned();
        assert_eq!(reversed.validate(&books), Err(ParamError::RangeReversed));
        // A report as at a date ignores the start date, as the web screen hides it.
        reversed.kind = ReportKind::TrialBalance;
        reversed.from = "not a date".to_owned();
        let point = reversed.validate(&books).unwrap();
        assert_eq!(point.from, point.to);

        let mut grant = form.clone();
        grant.kind = ReportKind::Grant;
        assert!(grant.validate(&books).is_ok());
        grant.grant = None;
        assert_eq!(grant.validate(&books), Err(ParamError::NoGrant));
        grant.grant = Some(GrantId::from("grant-none"));
        assert_eq!(
            grant.validate(&books),
            Err(ParamError::UnknownGrant(GrantId::from("grant-none")))
        );
        assert_eq!(
            ParamError::RangeReversed.to_string(),
            "The period starts after it ends. Choose a start date on or before the end date."
        );
    }

    #[test]
    fn presets_follow_the_fiscal_periods() {
        let got = presets(&demo(), &d(DEMO_AS_OF));
        let want = [
            ("month", "This month (Feb 2026)", "2026-02-01", "2026-02-04"),
            ("ytd", "Year to date", "2026-01-01", "2026-02-04"),
            ("last", "Fiscal year 2025", "2025-01-01", "2025-12-31"),
        ];
        assert_eq!(got.len(), want.len());
        for (p, (key, label, from, to)) in got.iter().zip(want) {
            assert_eq!(
                (p.key, p.label.as_str(), p.from.as_str(), p.to.as_str()),
                (key, label, from, to)
            );
        }
        // Empty books hold this year's periods and no earlier year.
        let empty = presets(&empty_books(), &d("2026-06-15"));
        let keys: Vec<_> = empty.iter().map(|p| p.key).collect();
        assert_eq!(keys, ["month", "ytd"]);
        assert_eq!(empty[0].to.as_str(), "2026-06-15");
    }

    #[test]
    fn money_shows_as_the_web_app_shows_it() {
        assert_eq!(format_minor(125_050), "1,250.50");
        assert_eq!(format_minor(-125_050), "(1,250.50)");
        assert_eq!(format_minor(0), "\u{2013}");
        assert_eq!(format_minor(5), "0.05");
        assert_eq!(format_minor(-99), "(0.99)");
        assert_eq!(format_minor(123_456_789_012), "1,234,567,890.12");
        assert_eq!(format_compact(1_570_000_000), "15.7M");
        assert_eq!(format_compact(1_500_000_000), "15M");
        assert_eq!(format_compact(85_000_000), "850K");
        assert_eq!(format_compact(950_000), "9,500");
        assert_eq!(format_compact(0), "0");
        assert_eq!(format_compact(-1_570_000_000), "\u{2212}15.7M");
    }

    #[test]
    fn dates_and_fixed_decimals_show_as_js_prints_them() {
        assert_eq!(format_date(&d("2026-02-04")), "4 Feb 2026");
        assert_eq!(format_date(&d("2025-12-31")), "31 Dec 2025");
        assert_eq!(format_date(&d("2026-09-30")), "30 Sept 2026");
        assert_eq!(to_fixed(0.031_25, 4), "0.0313");
        assert_eq!(to_fixed(-0.031_25, 4), "-0.0313");
        assert_eq!(to_fixed(2.5, 0), "3");
        assert_eq!(to_fixed(1.0, 4), "1.0000");
        assert_eq!(to_fixed(0.022_058_823_529_411_766, 4), "0.0221");
        assert_eq!(to_fixed(49.522_245_124_419_54, 1), "49.5");
        assert_eq!(date_from_days(0).as_str(), "1970-01-01");
        assert_eq!(date_from_days(19_723).as_str(), "2024-01-01");
        assert_eq!(date_from_days(19_782).as_str(), "2024-02-29");
        assert_eq!(date_from_days(-1).as_str(), "1969-12-31");
        assert!(today_utc().as_str() >= "2026-01-01");
    }

    #[test]
    fn trial_balance_rows_are_the_report_rows_and_a_total() {
        let books = demo();
        let p = params(
            ReportKind::TrialBalance,
            DEMO_AS_OF,
            DEMO_AS_OF,
            Basis::Accrual,
        );
        let s = statement(&books, &p).unwrap();
        let tb = reports::trial_balance(&books, Some(&p.to), Basis::Accrual);
        assert_eq!(s.title, "Trial balance");
        assert_eq!(s.period, "As at 4 Feb 2026");
        assert_eq!(
            (s.currency.as_str(), s.basis.as_str()),
            ("KES", "Accrual basis")
        );
        let table = s.tables().next().unwrap();
        assert_eq!(table.first_column, "Account");
        assert_eq!(table.columns, ["Debit", "Credit"]);
        assert_eq!(table.rows.len(), tb.rows.len() + 1);
        for (row, r) in table.rows.iter().zip(&tb.rows) {
            assert_eq!(row.kind, RowKind::Line);
            assert_eq!(row.code.as_deref(), Some(r.account.code.as_str()));
            assert_eq!(row.cells, [Cell::Money(r.debit), Cell::Money(r.credit)]);
        }
        let total = table.rows.last().unwrap();
        assert_eq!(
            (total.kind, total.label.as_str()),
            (RowKind::Total, "Totals")
        );
        assert_eq!(total.cells, [Cell::Money(tb.debit), Cell::Money(tb.credit)]);
        let footing = s.footings().next().unwrap();
        assert_eq!(footing.statement, "Total debits equal total credits.");
        assert!(footing.foots());
    }

    #[test]
    fn financial_position_groups_into_sections_with_subtotals() {
        let books = demo();
        let p = params(
            ReportKind::FinancialPosition,
            DEMO_AS_OF,
            DEMO_AS_OF,
            Basis::Cash,
        );
        let s = statement(&books, &p).unwrap();
        let fp = reports::financial_position(&books, &p.to, Basis::Cash);
        assert_eq!(s.basis, "Cash basis");
        let table = s.tables().next().unwrap();
        assert_eq!(table.columns, ["4 Feb 2026"]);
        assert_eq!(
            labels(table, RowKind::Heading),
            ["Assets", "Liabilities", "Net assets"]
        );
        assert_eq!(
            labels(table, RowKind::Subtotal),
            ["Total assets", "Total liabilities", "Total net assets"]
        );
        assert_eq!(
            labels(table, RowKind::Total),
            ["Total liabilities and net assets"]
        );
        let total = table.rows.last().unwrap();
        assert_eq!(
            total.cells,
            [Cell::Money(fp.total_liabilities + fp.total_net_assets)]
        );
        let lines = table
            .rows
            .iter()
            .filter(|r| r.kind == RowKind::Line)
            .count();
        // Assets, liabilities (or the "No liabilities" line), and the two net asset lines.
        assert_eq!(lines, fp.assets.len() + fp.liabilities.len().max(1) + 2);
        assert!(s.footings().all(Footing::foots));
    }

    #[test]
    fn empty_books_show_empty_states() {
        let books = empty_books();
        let tb = statement(
            &books,
            &params(
                ReportKind::TrialBalance,
                "2026-06-15",
                "2026-06-15",
                Basis::Accrual,
            ),
        )
        .unwrap();
        let table = tb.tables().next().unwrap();
        assert_eq!(
            table.empty.as_deref(),
            Some("No posted balances on or before 15 Jun 2026.")
        );
        let fp = statement(
            &books,
            &params(
                ReportKind::FinancialPosition,
                "2026-06-15",
                "2026-06-15",
                Basis::Accrual,
            ),
        )
        .unwrap();
        let none = fp
            .tables()
            .next()
            .unwrap()
            .rows
            .iter()
            .find(|r| r.label == "No liabilities")
            .unwrap();
        assert_eq!(none.cells, [Cell::Money(0)]);
        let act = statement(
            &books,
            &params(
                ReportKind::Activities,
                "2026-01-01",
                "2026-06-15",
                Basis::Accrual,
            ),
        )
        .unwrap();
        let empties: Vec<_> = act.tables().filter_map(|t| t.empty.as_deref()).collect();
        assert_eq!(
            empties,
            [
                "No income or expenses posted in this period.",
                "No fund had income or expenses in this period."
            ]
        );
    }

    #[test]
    fn activities_has_restriction_columns_and_a_fund_schedule() {
        let books = demo();
        let p = params(
            ReportKind::Activities,
            "2026-01-01",
            DEMO_AS_OF,
            Basis::Accrual,
        );
        let s = statement(&books, &p).unwrap();
        let a = reports::activities(&books, &p.from, &p.to, Basis::Accrual);
        assert_eq!(s.period, "For the period 1 Jan 2026 to 4 Feb 2026");
        let tables: Vec<_> = s.tables().collect();
        assert_eq!(tables.len(), 2);
        assert_eq!(
            tables[0].columns,
            ["Without restrictions", "With restrictions", "Total"]
        );
        assert_eq!(labels(tables[0], RowKind::Heading), ["Income", "Expenses"]);
        assert_eq!(
            labels(tables[0], RowKind::Subtotal),
            ["Total income", "Total expenses"]
        );
        let change = tables[0].rows.last().unwrap();
        assert_eq!(change.label, "Change in net assets");
        let c = &a.change_in_net_assets;
        assert_eq!(
            change.cells,
            [
                Cell::Money(c.unrestricted),
                Cell::Money(c.restricted),
                Cell::Money(c.total)
            ]
        );
        assert_eq!(tables[1].title.as_deref(), Some("Schedule by fund"));
        assert_eq!(tables[1].first_column, "Fund");
        assert_eq!(tables[1].columns, ["Income", "Expenses", "Change"]);
        assert_eq!(tables[1].rows.len(), a.by_fund.len() + 1);
        assert_eq!(
            tables[1].rows[0].detail.as_deref(),
            Some("Without donor restrictions")
        );
        assert_eq!(tables[1].rows.last().unwrap().label, "All funds");
        let footing = s.footings().next().unwrap();
        assert!(footing.foots(), "{footing:?}");
        assert!(
            footing
                .detail
                .as_deref()
                .unwrap()
                .ends_with("Year-end closing entries are left out.")
        );
    }

    #[test]
    fn functional_expenses_show_the_programme_ratio_and_shares() {
        let books = demo();
        let p = params(
            ReportKind::FunctionalExpenses,
            "2026-01-01",
            DEMO_AS_OF,
            Basis::Accrual,
        );
        let s = statement(&books, &p).unwrap();
        let fe = reports::functional_expenses(&books, &p.from, &p.to, Basis::Accrual);
        let Some(Block::Figures(figures)) = s.blocks.first() else {
            panic!("key figures first");
        };
        assert_eq!(figures[0].label, "Programme ratio");
        assert_eq!(
            figures[0].value,
            format!("{}%", to_fixed(fe.program_ratio * 100.0, 0))
        );
        let table = s.tables().next().unwrap();
        assert_eq!(
            table.columns,
            ["Programmes", "Management", "Fundraising", "Total"]
        );
        let share = table.rows.last().unwrap();
        assert_eq!(share.label, "Share of total");
        assert_eq!(share.cells.last(), Some(&Cell::Text("100%".to_owned())));
        assert!(s.footings().all(Footing::foots));
    }

    #[test]
    fn cash_flow_names_the_opening_and_closing_dates() {
        let books = demo();
        let p = params(ReportKind::CashFlow, "2026-01-01", DEMO_AS_OF, Basis::Cash);
        let s = statement(&books, &p).unwrap();
        assert_eq!(s.basis, "Direct method");
        let table = s.tables().next().unwrap();
        assert_eq!(
            labels(table, RowKind::Heading),
            [
                "Operating activities",
                "Investing activities",
                "Financing activities"
            ]
        );
        let cf = reports::cash_flow(&books, &p.from, &p.to).unwrap();
        let opening = table
            .rows
            .iter()
            .find(|r| r.label == "Cash at 31 Dec 2025")
            .unwrap();
        assert_eq!(opening.cells, [Cell::Money(cf.opening)]);
        let closing = table.rows.last().unwrap();
        assert_eq!(
            (closing.kind, closing.label.as_str()),
            (RowKind::Total, "Cash at 4 Feb 2026")
        );
        assert!(s.footings().all(Footing::foots));
    }

    #[test]
    fn the_grant_report_sets_out_the_summary() {
        let books = demo();
        let p = ReportParams {
            grant: Some(GrantId::from("grant-harambee-2026")),
            ..params(ReportKind::Grant, DEMO_AS_OF, DEMO_AS_OF, Basis::Accrual)
        };
        let s = statement(&books, &p).unwrap();
        let g =
            reports::grant_summary(&books, &GrantId::from("grant-harambee-2026"), &p.to).unwrap();
        assert_eq!(s.title, "Grant report: Harambee core support 2026");
        let table = s.tables().next().unwrap();
        let money: Vec<_> = table
            .rows
            .iter()
            .filter(|r| matches!(r.cells.first(), Some(Cell::Money(_))))
            .map(|r| (r.label.as_str(), r.cells[0].clone()))
            .collect();
        assert_eq!(
            money[..5],
            [
                ("Award", Cell::Money(g.award)),
                ("Received", Cell::Money(g.received)),
                ("Spent", Cell::Money(g.spent)),
                ("Committed", Cell::Money(g.committed)),
                ("Remaining", Cell::Money(g.remaining)),
            ]
        );
        assert!(s.footings().all(Footing::foots));
        let missing = ReportParams {
            grant: Some(GrantId::from("grant-none")),
            ..p
        };
        assert!(statement(&books, &missing).is_err());
    }

    #[test]
    fn csv_rows_open_with_the_title_block() {
        let books = demo();
        let p = params(
            ReportKind::TrialBalance,
            DEMO_AS_OF,
            DEMO_AS_OF,
            Basis::Accrual,
        );
        let rows = csv_rows(&books, &p).unwrap();
        let text = |r: &[CsvCell]| -> Vec<String> {
            r.iter()
                .map(|c| match c {
                    CsvCell::Text(s) => s.clone(),
                    other => format!("{other:?}"),
                })
                .collect()
        };
        assert_eq!(text(&rows[0]), ["Trial balance"]);
        assert_eq!(text(&rows[1]), ["As at 4 Feb 2026"]);
        assert_eq!(text(&rows[2]), ["Amounts in KES", "Accrual basis"]);
        assert_eq!(rows[3].len(), 0);
        assert_eq!(text(&rows[4]), ["Code", "Account", "Debit", "Credit"]);
        let tb = reports::trial_balance(&books, Some(&p.to), Basis::Accrual);
        assert_eq!(rows.len(), 5 + tb.rows.len() + 2);
        let first = &tb.rows[0];
        assert_eq!(
            text(&rows[5]),
            [
                first.account.code.clone(),
                first.account.name.clone(),
                ledger_core::money::format_major(first.debit),
                ledger_core::money::format_major(first.credit),
            ]
        );
        let csv = export_csv(&books, &p).unwrap();
        assert!(csv.starts_with("Trial balance\r\nAs at 4 Feb 2026\r\nAmounts in KES,Accrual basis\r\n\r\nCode,Account,Debit,Credit\r\n"));
    }

    #[test]
    fn export_files_are_named_as_the_web_download() {
        let tb = params(
            ReportKind::TrialBalance,
            DEMO_AS_OF,
            DEMO_AS_OF,
            Basis::Accrual,
        );
        assert_eq!(file_stem(&tb), "trial-balance_2026-02-04_accrual");
        let act = params(
            ReportKind::Activities,
            "2026-01-01",
            DEMO_AS_OF,
            Basis::Cash,
        );
        assert_eq!(file_stem(&act), "activities_2026-01-01_to_2026-02-04_cash");
        let cf = params(ReportKind::CashFlow, "2026-01-01", DEMO_AS_OF, Basis::Cash);
        assert_eq!(file_stem(&cf), "cash-flow_2026-01-01_to_2026-02-04");
        let grant = ReportParams {
            grant: Some(GrantId::from("grant-kisima-wash")),
            ..params(ReportKind::Grant, DEMO_AS_OF, DEMO_AS_OF, Basis::Accrual)
        };
        assert_eq!(file_stem(&grant), "grant_grant-kisima-wash_2026-02-04");
        assert_eq!(
            default_export_path(Some(Path::new("/home/someone")), &tb),
            PathBuf::from("/home/someone/Downloads/trial-balance_2026-02-04_accrual.csv")
        );
        assert_eq!(
            default_export_path(None, &tb),
            PathBuf::from("trial-balance_2026-02-04_accrual.csv")
        );
    }
}
