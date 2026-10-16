//! The General ledger screen's view models, free of GPUI: the filter bar's state and choices,
//! the table rows with their running balances, the totals row, the derived-rows cache and the
//! entry side panel.
//!
//! The rows are `ledger_core::reports::general_ledger` (`reports.ts:181-199`): every posted line
//! of the filter by date, then entry number, with its account's running balance after it. The
//! filter choices, the memo fallback, the debit and credit columns and the natural-side balance
//! follow the web screen (`flowtrace/src/routes/_authenticated/ledger/general.tsx`).

use std::collections::HashMap;
use std::sync::Arc;

use ledger_core::Minor;
use ledger_core::reports::{self, LineFilter, ReportError};
use ledger_core::types::{
    AccountId, AccountType, DimensionId, DimensionKind, EntryId, EntryStatus, FundId, GrantId,
    IsoDate, LedgerState, NormalBalance, PeriodId,
};

use crate::format;

/// What the filter bar holds. `None` means "all"; ids are those of the selected books.
#[derive(Debug, Clone, PartialEq, Eq, Default, Hash)]
pub struct GlFilter {
    /// An account and every account below it (`general.tsx`, "Header accounts include all of
    /// their sub-accounts").
    pub account: Option<AccountId>,
    /// Lines charged to this fund.
    pub fund: Option<FundId>,
    /// Lines charged to this grant.
    pub grant: Option<GrantId>,
    /// Lines tagged with this dimension value.
    pub dimension: Option<DimensionId>,
    /// A fiscal period: its first and last day bound the dates.
    pub period: Option<PeriodId>,
    /// First date, inclusive.
    pub from: Option<IsoDate>,
    /// Last date, inclusive.
    pub to: Option<IsoDate>,
}

impl GlFilter {
    /// The report filter for `state`: the ids as given (`general.tsx`, `buildLedger`), and the
    /// dates as the overlap of the period and `from`/`to`. A period not in `state` bounds
    /// nothing.
    #[must_use]
    pub fn to_line_filter(&self, state: &LedgerState) -> LineFilter {
        let _ = state;
        todo!()
    }

    /// Choosing `fund` drops a grant of another fund (`general.tsx`, the Fund select's
    /// `onValueChange`).
    pub fn set_fund(&mut self, fund: Option<FundId>, state: &LedgerState) {
        let _ = (fund, state);
        todo!()
    }

    /// True when nothing is filtered.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        *self == Self::default()
    }
}

/// One option of a filter select: the id it stands for (`None` for "All ...") and its label.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Choice<T> {
    /// The id, or `None` for the "All" option.
    pub value: Option<T>,
    /// What the select shows.
    pub label: String,
}

/// The options of each filter select for one set of books (`general.tsx`, `Filters`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FilterChoices {
    /// "All accounts", then every account by type (assets, liabilities, net assets, income,
    /// expenses) and code, as `1110 · Bank: Main operating account`, headers marked
    /// `(with sub-accounts)`.
    pub accounts: Vec<Choice<AccountId>>,
    /// "All funds", then the funds in book order.
    pub funds: Vec<Choice<FundId>>,
    /// "All grants", then the grants of the chosen fund (all grants with no fund chosen).
    pub grants: Vec<Choice<GrantId>>,
    /// "All tags", then dimension values by kind (projects, departments, locations, donors),
    /// as `Project · Name`.
    pub dimensions: Vec<Choice<DimensionId>>,
    /// "All periods", then the fiscal periods in book order.
    pub periods: Vec<Choice<PeriodId>>,
}

impl FilterChoices {
    /// The choices of `state` under `filter` (the grant list follows the chosen fund).
    #[must_use]
    pub fn new(state: &LedgerState, filter: &GlFilter) -> Self {
        let _ = (state, filter);
        todo!()
    }
}

/// The table's columns, in order.
pub const COLUMNS: [GlColumn; 9] = [
    GlColumn::Date,
    GlColumn::Entry,
    GlColumn::Account,
    GlColumn::Memo,
    GlColumn::Fund,
    GlColumn::Grant,
    GlColumn::Debit,
    GlColumn::Credit,
    GlColumn::Balance,
];

/// A column of the general ledger table.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GlColumn {
    /// Entry date, `4 Feb`.
    Date,
    /// Entry number, `JE-0001`.
    Entry,
    /// Account code and name.
    Account,
    /// The line's memo, else the entry's.
    Memo,
    /// Fund name.
    Fund,
    /// Grant name, or empty.
    Grant,
    /// Debit in base currency.
    Debit,
    /// Credit in base currency.
    Credit,
    /// The account's running balance after the line, natural side positive.
    Balance,
}

impl GlColumn {
    /// The header text.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Date => "Date",
            Self::Entry => "Entry",
            Self::Account => "Account",
            Self::Memo => "Memo",
            Self::Fund => "Fund",
            Self::Grant => "Grant",
            Self::Debit => "Debit",
            Self::Credit => "Credit",
            Self::Balance => "Balance",
        }
    }

    /// Amount columns are right-aligned.
    #[must_use]
    pub fn is_amount(self) -> bool {
        matches!(self, Self::Debit | Self::Credit | Self::Balance)
    }

    /// The column width in pixels.
    #[must_use]
    pub fn width(self) -> f32 {
        match self {
            Self::Date => 72.0,
            Self::Entry => 96.0,
            Self::Account => 220.0,
            Self::Memo => 280.0,
            Self::Fund => 160.0,
            Self::Grant => 180.0,
            Self::Debit | Self::Credit | Self::Balance => 128.0,
        }
    }
}

/// One row of the table: a posted line with its account's running balance.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GlRow {
    /// The entry, for the side panel and the Journal screen.
    pub entry_id: EntryId,
    /// Entry date.
    pub date: IsoDate,
    /// Entry number.
    pub entry_number: String,
    /// `code · name` of the account, shared between its rows.
    pub account: Arc<str>,
    /// The line's memo, else the entry's (`general.tsx`, `line.memo ?? line.entry.memo`).
    pub memo: String,
    /// Fund name (empty for a fund not in the books, as `general.tsx` `fund()`).
    pub fund: Arc<str>,
    /// Grant name, if the line is charged to a known grant.
    pub grant: Option<Arc<str>>,
    /// `max(0, base_amount)`.
    pub debit: Minor,
    /// `max(0, -base_amount)`.
    pub credit: Minor,
    /// The account's running balance after this line, natural side positive
    /// (`reports.ts:148-150`, `natural`).
    pub balance: Minor,
}

impl GlRow {
    /// The text of `column`: amounts as [`format::amount`] (an en dash for zero debits and
    /// credits), balances as [`format::balance`], dates as [`format::short_date`].
    #[must_use]
    pub fn cell(&self, column: GlColumn) -> String {
        let _ = column;
        todo!()
    }
}

/// The totals row and the key figures.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct GlTotals {
    /// Rows shown.
    pub lines: usize,
    /// Distinct accounts among them.
    pub accounts: usize,
    /// Sum of the debit column.
    pub debits: Minor,
    /// Sum of the credit column.
    pub credits: Minor,
    /// With one account filtered: its closing balance (with its sub-accounts), natural side
    /// positive by that account's normal balance. `None` otherwise, since balances of accounts
    /// on different sides do not add up.
    pub closing: Option<Minor>,
}

impl GlTotals {
    /// The text of `column` in the totals row.
    #[must_use]
    pub fn cell(&self, column: GlColumn) -> String {
        let _ = column;
        todo!()
    }
}

/// The derived table: rows, totals, and why there are none when the report refused.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct GlTable {
    /// Rows by date, then entry number.
    pub rows: Vec<GlRow>,
    /// The totals row.
    pub totals: GlTotals,
    /// The report's error (a `from` date not on the calendar), shown instead of rows.
    pub error: Option<String>,
}

impl GlTable {
    /// Derives the table of `state` under `filter`.
    #[must_use]
    pub fn derive(state: &LedgerState, filter: &GlFilter) -> Self {
        let _ = (state, filter);
        todo!()
    }

    /// Rows plus the totals row, as the table counts them (no totals row when there are no
    /// rows).
    #[must_use]
    pub fn rows_with_totals(&self) -> usize {
        todo!()
    }

    /// The text at `row_ix`, `column`; the row after the last is the totals row.
    #[must_use]
    pub fn cell(&self, row_ix: usize, column: GlColumn) -> String {
        let _ = (row_ix, column);
        todo!()
    }

    /// True for the totals row's index.
    #[must_use]
    pub fn is_totals_row(&self, row_ix: usize) -> bool {
        !self.rows.is_empty() && row_ix == self.rows.len()
    }
}

/// Keeps the last derived table and derives again only when the books' revision or the filter
/// changes, so a re-render never recomputes (ADR 0002, "Pattern for U1 to U10").
#[derive(Debug, Default)]
pub struct GlCache {
    key: Option<(u64, GlFilter)>,
    table: Arc<GlTable>,
    derivations: u64,
}

impl GlCache {
    /// The table for books `state` at `revision` under `filter`; derives it only when the pair
    /// differs from the last call. No books give an empty table.
    pub fn table(
        &mut self,
        revision: u64,
        state: Option<&LedgerState>,
        filter: &GlFilter,
    ) -> Arc<GlTable> {
        let _ = (revision, state, filter, &self.key, &self.table);
        todo!()
    }

    /// How many times a table was derived.
    #[must_use]
    pub fn derivations(&self) -> u64 {
        self.derivations
    }
}

/// One line of the entry side panel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DetailLine {
    /// `code · name`.
    pub account: String,
    /// Fund name, and the grant's after a comma when charged to one.
    pub fund: String,
    /// The line's own memo.
    pub memo: Option<String>,
    /// Base-currency debit.
    pub debit: Minor,
    /// Base-currency credit.
    pub credit: Minor,
}

/// The entry a clicked row belongs to, for the side panel until the Journal screen (U3) takes
/// over.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntryDetail {
    /// Entry id.
    pub id: EntryId,
    /// Entry number.
    pub number: String,
    /// Date.
    pub date: IsoDate,
    /// Memo.
    pub memo: String,
    /// `Posted`, `Reversed` or `Draft`.
    pub status: &'static str,
    /// Its lines in book order.
    pub lines: Vec<DetailLine>,
    /// Sum of the debits.
    pub debits: Minor,
    /// Sum of the credits.
    pub credits: Minor,
}

impl EntryDetail {
    /// The entry `id` of `state`, or `None` when it is not there.
    #[must_use]
    pub fn of(state: &LedgerState, id: &EntryId) -> Option<Self> {
        let _ = (state, id);
        todo!()
    }
}

#[cfg(test)]
mod tests {
    #![expect(clippy::unwrap_used, reason = "tests may unwrap")]

    use ledger_core::Ledger;
    use ledger_core::coa_template::{UNRESTRICTED_FUND_ID, account_id};
    use ledger_core::demo::build_demo_ledger;
    use ledger_core::engine::EntryOptions;
    use ledger_core::env::fixed;
    use ledger_core::setup::{create_org_ledger, system_actor};
    use ledger_core::types::{EntryInput, LineInput, Nullish};

    use super::*;

    fn line(code: &str, debit: Minor, credit: Minor) -> LineInput {
        #[expect(
            clippy::cast_precision_loss,
            reason = "a JS number input, as TS holds it"
        )]
        let side = |x: Minor| (x != 0).then_some(x as f64);
        LineInput {
            account_id: account_id(code),
            debit: side(debit),
            credit: side(credit),
            currency: None,
            fx_rate: None,
            fund_id: FundId::from(UNRESTRICTED_FUND_ID),
            grant_id: Nullish::Undefined,
            dimension_ids: None,
            flow_node_id: Nullish::Undefined,
            memo: Nullish::Undefined,
        }
    }

    fn entry(date: &str, memo: &str, lines: Vec<LineInput>) -> EntryInput {
        EntryInput {
            date: date.to_owned(),
            memo: memo.to_owned(),
            lines,
            source_type: None,
            source_id: Nullish::Undefined,
            override_reason: Nullish::Undefined,
        }
    }

    /// Two entries: supplies paid from the bank (10.00), then a donation into it (50.00).
    fn small_books() -> LedgerState {
        let mut env = fixed::test();
        let mut state = create_org_ledger("org", "KES", 2026, &mut env).unwrap();
        let mut ledger = Ledger::new(&mut state, &mut env);
        let actor = system_actor();
        ledger
            .post(
                &actor,
                &entry(
                    "2026-01-10",
                    "Office supplies",
                    vec![line("7100", 1_000, 0), line("1110", 0, 1_000)],
                ),
                EntryOptions::default(),
            )
            .unwrap();
        let mut gift = entry(
            "2026-02-03",
            "Donation",
            vec![line("1110", 5_000, 0), line("4200", 0, 5_000)],
        );
        gift.lines[1].memo = Nullish::Value("Gift from a supporter".to_owned());
        ledger.post(&actor, &gift, EntryOptions::default()).unwrap();
        state
    }

    fn date(s: &str) -> IsoDate {
        IsoDate::parse(s).unwrap()
    }

    #[test]
    fn an_empty_filter_is_an_empty_line_filter() {
        let books = small_books();
        assert_eq!(
            GlFilter::default().to_line_filter(&books),
            LineFilter::default()
        );
        assert!(GlFilter::default().is_empty());
    }

    #[test]
    fn the_filter_maps_ids_onto_the_line_filter() {
        let books = build_demo_ledger().unwrap();
        let filter = GlFilter {
            account: Some(account_id("1100")),
            fund: Some(FundId::from("fund-wash")),
            grant: Some(GrantId::from("grant-kisima-wash")),
            dimension: Some(DimensionId::from("dim-x")),
            ..GlFilter::default()
        };
        let lf = filter.to_line_filter(&books);
        assert_eq!(lf.account_ids, Some(vec![account_id("1100")]));
        assert_eq!(lf.fund_id, Some(FundId::from("fund-wash")));
        assert_eq!(lf.grant_id, Some(GrantId::from("grant-kisima-wash")));
        assert_eq!(lf.dimension_id, Some(DimensionId::from("dim-x")));
        assert_eq!((lf.from, lf.to), (None, None));
    }

    #[test]
    fn a_period_bounds_the_dates_and_overlaps_from_and_to() {
        let books = build_demo_ledger().unwrap();
        let period = GlFilter {
            period: Some(PeriodId::from("period-2026-01")),
            ..GlFilter::default()
        };
        let lf = period.to_line_filter(&books);
        assert_eq!(lf.from, Some(date("2026-01-01")));
        assert_eq!(lf.to, Some(date("2026-01-31")));

        let narrowed = GlFilter {
            from: Some(date("2026-01-15")),
            to: Some(date("2026-03-01")),
            ..period.clone()
        };
        let lf = narrowed.to_line_filter(&books);
        assert_eq!(lf.from, Some(date("2026-01-15")));
        assert_eq!(lf.to, Some(date("2026-01-31")));

        let unknown = GlFilter {
            period: Some(PeriodId::from("period-1999-01")),
            to: Some(date("2026-03-01")),
            ..GlFilter::default()
        };
        let lf = unknown.to_line_filter(&books);
        assert_eq!((lf.from, lf.to), (None, Some(date("2026-03-01"))));
    }

    #[test]
    fn choosing_another_fund_drops_a_grant_of_the_old_one() {
        let books = build_demo_ledger().unwrap();
        let wash = books
            .grants
            .iter()
            .find(|g| g.id.as_str() == "grant-kisima-wash")
            .unwrap();
        let mut filter = GlFilter {
            grant: Some(wash.id.clone()),
            ..GlFilter::default()
        };
        filter.set_fund(Some(wash.fund_id.clone()), &books);
        assert_eq!(filter.grant, Some(wash.id.clone()));
        filter.set_fund(None, &books);
        assert_eq!(filter.grant, Some(wash.id.clone()));
        filter.set_fund(Some(FundId::from(UNRESTRICTED_FUND_ID)), &books);
        assert_eq!(filter.grant, None);
    }

    #[test]
    fn choices_list_all_first_then_the_books() {
        let books = build_demo_ledger().unwrap();
        let choices = FilterChoices::new(&books, &GlFilter::default());

        assert_eq!(
            choices.accounts[0],
            Choice {
                value: None,
                label: "All accounts".to_owned()
            }
        );
        assert_eq!(choices.accounts.len(), books.accounts.len() + 1);
        let labels: Vec<_> = choices.accounts.iter().map(|c| c.label.as_str()).collect();
        assert!(labels.contains(&"1110 \u{b7} Bank: Main operating account"));
        assert!(labels.contains(&"1100 \u{b7} Cash and cash equivalents (with sub-accounts)"));
        // Assets before liabilities before income before expenses, by code within a type.
        let pos = |code: &str| labels.iter().position(|l| l.starts_with(code)).unwrap();
        assert!(pos("1110") < pos("1120"));
        assert!(pos("1590") < pos("2100"));
        assert!(pos("3200") < pos("4100"));
        assert!(pos("4400") < pos("5100"));

        assert_eq!(choices.funds[0].label, "All funds");
        assert_eq!(choices.funds.len(), books.funds.len() + 1);
        assert_eq!(choices.grants[0].label, "All grants");
        assert_eq!(choices.grants.len(), books.grants.len() + 1);
        assert_eq!(choices.dimensions[0].label, "All tags");
        assert_eq!(choices.dimensions.len(), books.dimensions.len() + 1);
        assert_eq!(choices.periods[0].label, "All periods");
        assert_eq!(choices.periods.len(), books.periods.len() + 1);
        assert_eq!(choices.periods[1].value, Some(books.periods[0].id.clone()));

        let wash = FundId::from("fund-wash");
        let only_wash = FilterChoices::new(
            &books,
            &GlFilter {
                fund: Some(wash.clone()),
                ..GlFilter::default()
            },
        );
        let expected = 1 + books.grants.iter().filter(|g| g.fund_id == wash).count();
        assert_eq!(only_wash.grants.len(), expected);
    }

    #[test]
    fn dimension_choices_go_by_kind_with_a_kind_label() {
        let books = build_demo_ledger().unwrap();
        let choices = FilterChoices::new(&books, &GlFilter::default());
        let kinds: Vec<_> = choices.dimensions[1..]
            .iter()
            .map(|c| {
                let id = c.value.as_ref().unwrap();
                books.dimensions.iter().find(|d| &d.id == id).unwrap().kind
            })
            .collect();
        let rank = |k: &DimensionKind| match k {
            DimensionKind::Project => 0,
            DimensionKind::Department => 1,
            DimensionKind::Location => 2,
            DimensionKind::Donor => 3,
        };
        assert!(kinds.windows(2).all(|w| rank(&w[0]) <= rank(&w[1])));
        let first = &books
            .dimensions
            .iter()
            .find(|d| d.kind == kinds[0])
            .unwrap();
        let kind = match first.kind {
            DimensionKind::Project => "Project",
            DimensionKind::Department => "Department",
            DimensionKind::Location => "Location",
            DimensionKind::Donor => "Donor",
        };
        assert_eq!(
            choices.dimensions[1].label,
            format!("{kind} \u{b7} {}", first.name)
        );
    }

    #[test]
    fn rows_follow_the_report_with_natural_running_balances() {
        let books = small_books();
        let table = GlTable::derive(&books, &GlFilter::default());
        let summary: Vec<_> = table
            .rows
            .iter()
            .map(|r| (r.date.as_str(), &*r.account, r.debit, r.credit, r.balance))
            .collect();
        assert_eq!(
            summary,
            [
                ("2026-01-10", "7100 \u{b7} Office supplies", 1_000, 0, 1_000),
                (
                    "2026-01-10",
                    "1110 \u{b7} Bank: Main operating account",
                    0,
                    1_000,
                    -1_000
                ),
                (
                    "2026-02-03",
                    "1110 \u{b7} Bank: Main operating account",
                    5_000,
                    0,
                    4_000
                ),
                // Income is a credit account: its balance shows positive.
                ("2026-02-03", "4200 \u{b7} Donations", 0, 5_000, 5_000),
            ]
        );
        assert_eq!(table.rows[3].memo, "Gift from a supporter");
        assert_eq!(table.rows[2].memo, "Donation");
        assert_eq!(&*table.rows[0].fund, "Unrestricted");
        assert_eq!(table.rows[0].grant, None);
        assert_eq!(table.error, None);
    }

    #[test]
    fn rows_match_reports_general_ledger_on_the_demo_books() {
        let books = build_demo_ledger().unwrap();
        let filter = GlFilter::default();
        let report = reports::general_ledger(&books, &filter.to_line_filter(&books)).unwrap();
        let table = GlTable::derive(&books, &filter);
        assert_eq!(table.rows.len(), report.rows.len());
        for (row, r) in table.rows.iter().zip(&report.rows) {
            let account = r.account.as_ref().unwrap();
            assert_eq!(row.entry_id, r.line.entry.id);
            assert_eq!(row.debit - row.credit, r.line.line.base_amount);
            assert_eq!(
                row.balance,
                reports::natural(account.normal_balance, r.running)
            );
        }
    }

    #[test]
    fn cells_format_amounts_dates_and_blanks() {
        let books = small_books();
        let table = GlTable::derive(&books, &GlFilter::default());
        let bank = &table.rows[1];
        assert_eq!(bank.cell(GlColumn::Date), "10 Jan");
        assert_eq!(bank.cell(GlColumn::Entry), "JE-0001");
        assert_eq!(
            bank.cell(GlColumn::Account),
            "1110 \u{b7} Bank: Main operating account"
        );
        assert_eq!(bank.cell(GlColumn::Memo), "Office supplies");
        assert_eq!(bank.cell(GlColumn::Fund), "Unrestricted");
        assert_eq!(bank.cell(GlColumn::Grant), "");
        assert_eq!(bank.cell(GlColumn::Debit), "\u{2013}");
        assert_eq!(bank.cell(GlColumn::Credit), "10.00");
        assert_eq!(bank.cell(GlColumn::Balance), "(10.00)");
        assert_eq!(table.cell(2, GlColumn::Balance), "40.00");
    }

    #[test]
    fn the_totals_row_sums_the_columns() {
        let books = small_books();
        let table = GlTable::derive(&books, &GlFilter::default());
        assert_eq!(
            table.totals,
            GlTotals {
                lines: 4,
                accounts: 3,
                debits: 6_000,
                credits: 6_000,
                closing: None
            }
        );
        assert_eq!(table.rows_with_totals(), 5);
        assert!(table.is_totals_row(4));
        assert!(!table.is_totals_row(3));
        assert_eq!(table.cell(4, GlColumn::Date), "Totals");
        assert_eq!(table.cell(4, GlColumn::Entry), "4 lines");
        assert_eq!(table.cell(4, GlColumn::Debit), "60.00");
        assert_eq!(table.cell(4, GlColumn::Credit), "60.00");
        assert_eq!(table.cell(4, GlColumn::Balance), "");
        assert_eq!(table.cell(9, GlColumn::Date), "");
    }

    #[test]
    fn one_account_closes_on_its_natural_side_with_sub_accounts() {
        let books = small_books();
        let bank = GlTable::derive(
            &books,
            &GlFilter {
                account: Some(account_id("1110")),
                ..GlFilter::default()
            },
        );
        assert_eq!(bank.totals.closing, Some(4_000));
        assert_eq!(bank.cell(2, GlColumn::Balance), "40.00");

        let income = GlTable::derive(
            &books,
            &GlFilter {
                account: Some(account_id("4000")),
                ..GlFilter::default()
            },
        );
        assert_eq!(income.rows.len(), 1);
        assert_eq!(income.totals.closing, Some(5_000));

        let demo = build_demo_ledger().unwrap();
        let cash = GlFilter {
            account: Some(account_id("1100")),
            ..GlFilter::default()
        };
        let table = GlTable::derive(&demo, &cash);
        let balances = reports::balances(&demo, &cash.to_line_filter(&demo));
        assert_eq!(table.totals.closing, Some(balances.values().sum::<Minor>()));
        assert!(table.totals.accounts > 1);
    }

    #[test]
    fn dates_start_from_the_opening_balance() {
        let books = small_books();
        let table = GlTable::derive(
            &books,
            &GlFilter {
                from: Some(date("2026-02-01")),
                ..GlFilter::default()
            },
        );
        assert_eq!(table.rows.len(), 2);
        // The bank opened February at (10.00) and closes it at 40.00.
        assert_eq!(table.rows[0].balance, 4_000);
        assert_eq!(table.totals.debits, 5_000);
    }

    #[test]
    fn a_report_error_shows_instead_of_rows() {
        let books = small_books();
        let table = GlTable::derive(
            &books,
            &GlFilter {
                from: Some(date("2026-13-01")),
                ..GlFilter::default()
            },
        );
        assert!(table.rows.is_empty());
        assert_eq!(table.rows_with_totals(), 0);
        assert_eq!(
            table.error.as_deref(),
            Some("2026-13-01 is not a calendar date.")
        );
    }

    #[test]
    fn the_cache_derives_once_per_revision_and_filter() {
        let books = small_books();
        let mut cache = GlCache::default();
        let filter = GlFilter::default();
        let first = cache.table(1, Some(&books), &filter);
        let again = cache.table(1, Some(&books), &filter);
        assert!(Arc::ptr_eq(&first, &again));
        assert_eq!(cache.derivations(), 1);

        let bank = GlFilter {
            account: Some(account_id("1110")),
            ..GlFilter::default()
        };
        assert_eq!(cache.table(1, Some(&books), &bank).rows.len(), 2);
        assert_eq!(cache.derivations(), 2);
        assert_eq!(cache.table(2, Some(&books), &bank).rows.len(), 2);
        assert_eq!(cache.derivations(), 3);
        assert!(cache.table(3, None, &bank).rows.is_empty());
        assert_eq!(cache.derivations(), 4);
    }

    #[test]
    fn the_entry_detail_lists_its_lines() {
        let books = small_books();
        let table = GlTable::derive(&books, &GlFilter::default());
        let detail = EntryDetail::of(&books, &table.rows[3].entry_id).unwrap();
        assert_eq!(detail.number, "JE-0002");
        assert_eq!(detail.date, date("2026-02-03"));
        assert_eq!(detail.memo, "Donation");
        assert_eq!(detail.status, "Posted");
        assert_eq!(
            detail.lines,
            [
                DetailLine {
                    account: "1110 \u{b7} Bank: Main operating account".to_owned(),
                    fund: "Unrestricted".to_owned(),
                    memo: None,
                    debit: 5_000,
                    credit: 0,
                },
                DetailLine {
                    account: "4200 \u{b7} Donations".to_owned(),
                    fund: "Unrestricted".to_owned(),
                    memo: Some("Gift from a supporter".to_owned()),
                    debit: 0,
                    credit: 5_000,
                },
            ]
        );
        assert_eq!((detail.debits, detail.credits), (5_000, 5_000));
        assert_eq!(EntryDetail::of(&books, &EntryId::from("missing")), None);
    }

    #[test]
    fn a_grant_line_names_its_grant_in_the_row_and_the_detail() {
        let books = build_demo_ledger().unwrap();
        let table = GlTable::derive(
            &books,
            &GlFilter {
                grant: Some(GrantId::from("grant-kisima-wash")),
                ..GlFilter::default()
            },
        );
        assert!(!table.rows.is_empty());
        let grant = books
            .grants
            .iter()
            .find(|g| g.id.as_str() == "grant-kisima-wash")
            .unwrap();
        assert!(
            table
                .rows
                .iter()
                .all(|r| r.grant.as_deref() == Some(grant.name.as_str()))
        );
        let detail = EntryDetail::of(&books, &table.rows[0].entry_id).unwrap();
        assert!(
            detail
                .lines
                .iter()
                .any(|l| l.fund.ends_with(&format!(", {}", grant.name)))
        );
    }
}
