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

use std::fmt;
use std::path::{Path, PathBuf};

use ledger_core::Minor;
use ledger_core::csv::CsvCell;
use ledger_core::reports::{Basis, ReportError};
use ledger_core::types::{GrantId, IsoDate, LedgerState};

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
    /// `as_of` (the web screen's `ytd` preset), accrual basis and the first grant.
    #[must_use]
    pub fn new(state: &LedgerState, as_of: &IsoDate) -> Self {
        let _ = (state, as_of);
        Self {
            kind: ReportKind::default(),
            from: String::new(),
            to: String::new(),
            basis: Basis::Accrual,
            grant: None,
        }
    }

    /// Checks the dates and the grant against `state`.
    ///
    /// # Errors
    ///
    /// [`ParamError`] naming the first field that is wrong.
    pub fn validate(&self, state: &LedgerState) -> Result<ReportParams, ParamError> {
        let _ = state;
        Err(ParamError::To)
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

/// The period presets for `as_of`: this month, year to date and the previous fiscal year.
#[must_use]
pub fn presets(state: &LedgerState, as_of: &IsoDate) -> Vec<Preset> {
    let _ = (state, as_of);
    Vec::new()
}

/// One cell of a statement row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Cell {
    /// An amount in minor units, shown with [`format_minor`].
    Money(Minor),
    /// Text as it stands, such as a percentage.
    Text(String),
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
    /// Shown instead of the table when it has nothing to show.
    pub empty: Option<String>,
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

/// A statement ready to draw (`ReportBody`): its title block and its blocks.
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

/// The statement for `params` over `state`.
///
/// # Errors
///
/// [`ReportError`] where the report function refuses the parameters.
pub fn statement(state: &LedgerState, params: &ReportParams) -> Result<Statement, ReportError> {
    let _ = state;
    Ok(Statement {
        kind: params.kind,
        title: String::new(),
        period: String::new(),
        currency: String::new(),
        basis: String::new(),
        blocks: Vec::new(),
    })
}

/// The report as spreadsheet rows, amounts in major units (`reportCsv`).
///
/// # Errors
///
/// [`ReportError`] where the report function refuses the parameters.
pub fn csv_rows(
    state: &LedgerState,
    params: &ReportParams,
) -> Result<Vec<Vec<CsvCell>>, ReportError> {
    let _ = (state, params);
    Ok(Vec::new())
}

/// The report's CSV text through `ledger_core::csv::to_csv`.
///
/// # Errors
///
/// [`ReportError`] where the report function refuses the parameters.
pub fn export_csv(state: &LedgerState, params: &ReportParams) -> Result<String, ReportError> {
    let _ = (state, params);
    Ok(String::new())
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

/// Writes the report's CSV to `path`.
///
/// # Errors
///
/// [`ExportError`] when the report refuses its parameters or the file cannot be written.
pub fn write_export(
    state: &LedgerState,
    params: &ReportParams,
    path: &Path,
) -> Result<(), ExportError> {
    let _ = (state, params, path);
    Ok(())
}

/// The export's file name without extension, as the web screen names its download:
/// `<id>_<date or from_to_to>[_<basis>]`.
#[must_use]
pub fn file_stem(params: &ReportParams) -> String {
    let _ = params;
    String::new()
}

/// `<home>/Downloads/<file_stem>.csv`, or the bare file name when there is no home.
#[must_use]
pub fn default_export_path(home: Option<&Path>, params: &ReportParams) -> PathBuf {
    let _ = (home, params);
    PathBuf::new()
}

/// An amount for display (`formatMinor` in `flowtrace/src/components/ledger/money.tsx`): two
/// decimals with thousands separators, negatives in parentheses, zero as an en dash.
#[must_use]
pub fn format_minor(minor: Minor) -> String {
    let _ = minor;
    String::new()
}

/// A compact figure for headline numbers (`formatMinorCompact`): 15.7M, 850K, 9,500.
#[must_use]
pub fn format_compact(minor: Minor) -> String {
    let _ = minor;
    String::new()
}

/// A date as the web app prints it (`formatDate` in `flowtrace/src/lib/format.ts`, `en-GB`
/// day, short month, year): `4 Feb 2026`.
#[must_use]
pub fn format_date(date: &IsoDate) -> String {
    let _ = date;
    String::new()
}

/// `Number.prototype.toFixed(digits)`: rounded from the exact binary value, ties away from zero.
#[must_use]
pub fn to_fixed(x: f64, digits: usize) -> String {
    let _ = (x, digits);
    String::new()
}

/// The calendar date `days` after 1970-01-01.
#[must_use]
pub fn date_from_days(days: i64) -> IsoDate {
    let _ = days;
    IsoDate::parse("1970-01-01").unwrap_or_else(|_| unreachable!())
}

/// Today in UTC, the as-at date for books that are not the demo's.
#[must_use]
pub fn today_utc() -> IsoDate {
    date_from_days(0)
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
        assert!(rows[3].is_empty());
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
