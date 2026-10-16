//! The Grants screen's view models (`desktop::grants`) on the demo books, and the add, set and
//! change commands driven through the in-process backend. Expected figures are the TS engine's
//! (`rust/fixtures/report-grant-summary.json` and `report-budget-vs-actual.json`, demo books as at
//! 2026-02-04); wording is the web screens' (`flowtrace/src/routes/_authenticated/grants/`).

#![expect(clippy::unwrap_used, reason = "tests may unwrap")]

use desktop::accounts::Failure;
use desktop::backend::{InProcessBackend, LedgerBackend as _};
use desktop::grants::{
    BarKind, BudgetFilter, BudgetLineForm, BudgetRow, Group, LineEdit, NewGrantForm, budget_view,
    expense_choices, grant_detail, grant_rows, grant_status_text, grant_totals, line_status_text,
    months, share_percent, totals_figures, used_percent,
};
use desktop::statements::{Cell, RowKind};
use ledger_core::coa_template::{UNRESTRICTED_FUND_ID, account_id};
use ledger_core::demo::{DEMO_AS_OF, DEMO_ORG_ID, build_demo_ledger};
use ledger_core::reports::{Basis, WarningLevel};
use ledger_core::types::{
    AccountId, BudgetId, DimensionId, EntryInput, FundId, GrantId, IsoDate, LedgerState, LineInput,
    Nullish, OrganizationId,
};
use ledger_core::{CommandResult, LedgerCommand};

fn as_of() -> IsoDate {
    IsoDate::parse(DEMO_AS_OF).unwrap()
}

fn demo() -> OrganizationId {
    OrganizationId::from(DEMO_ORG_ID)
}

fn backend() -> InProcessBackend {
    InProcessBackend::with_demo_and_empty().unwrap()
}

fn books(backend: &InProcessBackend) -> std::sync::Arc<LedgerState> {
    backend.state(&demo()).unwrap()
}

fn grant(id: &str) -> GrantId {
    GrantId::from(id)
}

// Wording and rounding.

#[test]
fn status_texts_follow_the_web() {
    assert_eq!(grant_status_text(WarningLevel::Within), "Within award");
    assert_eq!(grant_status_text(WarningLevel::Warning), "80% used");
    assert_eq!(grant_status_text(WarningLevel::Over), "Over award");
    assert_eq!(line_status_text(WarningLevel::Within), "Within budget");
    assert_eq!(line_status_text(WarningLevel::Warning), "80% or more used");
    assert_eq!(line_status_text(WarningLevel::Over), "Over budget");
}

#[test]
fn percentages_round_as_math_round() {
    assert_eq!(share_percent(121_230_000, 320_000_000), 38);
    // Math.max(1, award): no division by zero.
    assert_eq!(share_percent(0, 0), 0);
    // Ties round up, as Math.round does: 0.125 * 100 = 12.5.
    assert_eq!(share_percent(125, 1000), 13);
    assert_eq!(used_percent(0.735_294_117_647_058_9), Some(74));
    assert_eq!(used_percent(1.11), Some(111));
    assert_eq!(used_percent(f64::INFINITY), None);
}

#[test]
fn months_read_as_the_web_writes_them() {
    assert_eq!(months(0.670_225_872_866_096_4), "0.7 months");
    assert_eq!(months(1.0), "1 month");
    assert_eq!(months(9.0), "9 months");
    assert_eq!(months(9.010_373_312_288_143), "9 months");
    assert_eq!(months(10.8), "11 months");
    assert_eq!(months(16.118_275_154), "16 months");
}

// The list.

#[test]
fn the_list_is_most_used_first_with_the_engine_figures() {
    let books = build_demo_ledger().unwrap();
    let rows = grant_rows(&books, &as_of()).unwrap();
    let ids: Vec<&str> = rows.iter().map(|r| r.grant.id.as_str()).collect();
    assert_eq!(
        ids,
        [
            "grant-kisima-wash",
            "grant-northlight-2026",
            "grant-paef-education",
            "grant-harambee-2026"
        ]
    );
    let kisima = &rows[0];
    assert_eq!(kisima.summary.spent, 250_000_000);
    assert_eq!(kisima.summary.committed, 1_850_000);
    assert_eq!(kisima.summary.remaining, 148_150_000);
    assert_eq!(kisima.fund, "Water and sanitation (restricted)");
    assert_eq!(
        kisima.subtitle(),
        "Kisima Water Trust \u{b7} Water and sanitation (restricted) \u{b7} 1 Jan 2026 to 30 Jun 2027"
    );
    assert_eq!(
        kisima.status_line(),
        "Within award \u{b7} 63% spent or committed"
    );
    assert_eq!(kisima.coverage_label(), "63% spent, 0% committed");
    assert_eq!(rows[2].coverage_label(), "0% spent, 3% committed");
}

#[test]
fn the_totals_add_every_grant() {
    let books = build_demo_ledger().unwrap();
    let rows = grant_rows(&books, &as_of()).unwrap();
    let t = grant_totals(&rows);
    assert_eq!(
        t.award,
        3_400_000_000 + 320_000_000 + 400_000_000 + 600_000_000
    );
    assert_eq!(t.spent, 75_000_000 + 121_230_000 + 250_000_000);
    assert_eq!(t.committed, 1_850_000 + 18_600_000);
    assert_eq!(t.remaining, t.award - t.spent - t.committed);
    let figures = totals_figures(&t, "KES");
    let pairs: Vec<(&str, &str)> = figures
        .iter()
        .map(|f| (f.value.as_str(), f.label.as_str()))
        .collect();
    assert_eq!(
        pairs,
        [
            ("47.2M", "Awarded, KES"),
            ("4.5M", "Spent"),
            ("205K", "Committed"),
            ("42.5M", "Remaining")
        ]
    );
}

#[test]
fn no_grants_no_rows() {
    let backend = backend();
    let empty = &backend.organisations()[1].id;
    let books = backend.state(empty).unwrap();
    assert!(grant_rows(&books, &as_of()).unwrap().is_empty());
}

/// Adds a grant of 1,000.00 on the unrestricted fund to the demo books and returns its id.
fn small_grant(backend: &mut InProcessBackend) -> GrantId {
    let mut form = NewGrantForm::new(&books(backend));
    form.name = "Threshold test".to_owned();
    form.donor = "Test donor".to_owned();
    form.fund = Some(FundId::from(UNRESTRICTED_FUND_ID));
    form.award = "1000".to_owned();
    form.start = "2026-01-01".to_owned();
    form.end = "2026-12-31".to_owned();
    let result = backend.submit(&demo(), form.command().unwrap()).unwrap();
    let CommandResult::Id { id } = result else {
        panic!("grant.add returns an id, got {result:?}");
    };
    GrantId::from(id.as_str())
}

/// Posts `minor` of office supplies charged to `grant`, paid from cash.
fn spend(backend: &mut InProcessBackend, grant: &GrantId, minor: i64) {
    #[expect(clippy::cast_precision_loss, reason = "small test amounts")]
    let amount = Some(minor as f64);
    let line = |code: &str, debit: bool| LineInput {
        account_id: account_id(code),
        debit: if debit { amount } else { None },
        credit: if debit { None } else { amount },
        currency: None,
        fx_rate: None,
        fund_id: FundId::from(UNRESTRICTED_FUND_ID),
        grant_id: if debit {
            Nullish::Value(grant.clone())
        } else {
            Nullish::Undefined
        },
        dimension_ids: None,
        flow_node_id: Nullish::Undefined,
        memo: Nullish::Undefined,
    };
    let input = EntryInput {
        date: "2026-02-03".to_owned(),
        memo: "Threshold spend".to_owned(),
        lines: vec![line("7100", true), line("1110", false)],
        source_type: None,
        source_id: Nullish::Undefined,
        // Going over the award is refused without a reason (GRANT_OVER_BUDGET, overridable).
        override_reason: Nullish::Value("Threshold test".to_owned()),
    };
    backend
        .submit(&demo(), LedgerCommand::EntryPost { input })
        .unwrap();
}

fn row_of(backend: &InProcessBackend, id: &GrantId) -> desktop::grants::GrantRow {
    grant_rows(&books(backend), &as_of())
        .unwrap()
        .into_iter()
        .find(|r| &r.grant.id == id)
        .unwrap()
}

#[test]
fn warnings_start_at_exactly_80_percent_and_over_is_past_100() {
    let mut backend = backend();
    let id = small_grant(&mut backend);
    assert_eq!(row_of(&backend, &id).summary.status, WarningLevel::Within);

    spend(&mut backend, &id, 79_999);
    assert_eq!(row_of(&backend, &id).summary.status, WarningLevel::Within);

    spend(&mut backend, &id, 1);
    let at_80 = row_of(&backend, &id);
    assert_eq!(at_80.summary.status, WarningLevel::Warning);
    assert_eq!(
        at_80.status_line(),
        "80% used \u{b7} 80% spent or committed"
    );

    spend(&mut backend, &id, 20_000);
    let at_100 = row_of(&backend, &id);
    assert_eq!(at_100.summary.status, WarningLevel::Warning);
    assert_eq!(
        at_100.status_line(),
        "80% used \u{b7} 100% spent or committed"
    );

    spend(&mut backend, &id, 1);
    let over = row_of(&backend, &id);
    assert_eq!(over.summary.status, WarningLevel::Over);
    assert_eq!(
        over.status_line(),
        "Over award \u{b7} 100% spent or committed"
    );
    assert_eq!(over.summary.remaining, -1);
}

// The detail.

#[test]
fn the_detail_maps_the_grant_summary() {
    let books = build_demo_ledger().unwrap();
    let d = grant_detail(&books, &grant("grant-northlight-2026"), &as_of()).unwrap();
    assert_eq!(d.summary.spent, 121_230_000);
    assert_eq!(d.summary.burn_per_month, 108_527_592);
    assert_eq!(
        d.lede,
        "Northlight Foundation \u{b7} Maternal health (restricted) \u{b7} 1 Jan 2026 to 31 Dec 2026"
    );
    let figures: Vec<(&str, &str)> = d
        .figures
        .iter()
        .map(|f| (f.value.as_str(), f.label.as_str()))
        .collect();
    assert_eq!(
        figures,
        [
            ("3.2M", "Awarded, KES"),
            ("1.2M", "Spent"),
            ("0", "Committed"),
            ("2M", "Remaining")
        ]
    );
    let more: Vec<(&str, &str)> = d
        .more
        .iter()
        .map(|f| (f.value.as_str(), f.label.as_str()))
        .collect();
    assert_eq!(
        more,
        [
            ("3.2M", "Received from the donor"),
            ("1.1M", "Spent per month, on average"),
            ("1.8 months", "Money lasts, at that rate"),
            ("11 months", "Until the grant ends")
        ]
    );
    assert_eq!(d.coverage_label, "38% spent, 0% committed");
    assert!(d.runway.flag);
    assert_eq!(
        d.runway.text,
        "At this rate the award runs out in about 1.8 months, 9 months before the grant ends on \
         31 Dec 2026."
    );
    // Budget lines for this grant only, in book order.
    let lines: Vec<&str> = d.budget.iter().map(|r| r.line.id.as_str()).collect();
    assert_eq!(lines, ["demo-003b", "demo-003c"]);
    // Posted lines charged to it, newest first, closing entries left out.
    assert!(!d.lines.is_empty());
    assert!(
        d.lines
            .windows(2)
            .all(|w| (&w[0].date, &w[0].entry_number) >= (&w[1].date, &w[1].entry_number))
    );
    let debits: i64 = d
        .lines
        .iter()
        .filter(|l| l.code.starts_with('5') || l.code.starts_with('6') || l.code.starts_with('7'))
        .map(|l| l.debit - l.credit)
        .sum();
    assert_eq!(debits, d.summary.spent);
}

#[test]
fn the_runway_reads_each_case() {
    let books = build_demo_ledger().unwrap();
    let detail = |id: &str| grant_detail(&books, &grant(id), &as_of()).unwrap();
    let harambee = detail("grant-harambee-2026");
    assert!(harambee.runway.flag);
    assert_eq!(
        harambee.runway.text,
        "At this rate about 76% of the award (26M) is still unspent when the grant ends on \
         31 Dec 2026."
    );
    let paef = detail("grant-paef-education");
    assert!(!paef.runway.flag);
    assert_eq!(
        paef.runway.text,
        "No spend is posted yet, so there is no rate to project from."
    );
    assert_eq!(paef.more[2].value, "\u{2013}");
    assert_eq!(
        detail("grant-kisima-wash").runway.text,
        "At this rate the award runs out in about 0.7 months, 16 months before the grant ends \
         on 30 Jun 2027."
    );
    let ended = grant_detail(
        &books,
        &grant("grant-harambee-2026"),
        &IsoDate::parse("2027-03-01").unwrap(),
    )
    .unwrap();
    assert!(!ended.runway.flag);
    assert_eq!(ended.runway.text, "The grant ended on 31 Dec 2026.");
}

#[test]
fn an_unknown_grant_has_no_detail() {
    let books = build_demo_ledger().unwrap();
    assert!(grant_detail(&books, &grant("nope"), &as_of()).is_err());
}

// The budget page.

fn filter() -> BudgetFilter {
    BudgetFilter::default()
}

#[test]
fn the_budget_rows_group_grant_running_department() {
    let books = build_demo_ledger().unwrap();
    let view = budget_view(&books, &filter(), &as_of()).unwrap();
    assert_eq!(view.budget.id.as_str(), "budget-2026");
    assert_eq!(view.rows.len(), 15);
    let groups: Vec<Group> = view.rows.iter().map(|r| r.group).collect();
    assert!(groups[..8].iter().all(|g| *g == Group::Grant));
    assert!(groups[8..14].iter().all(|g| *g == Group::Running));
    assert_eq!(groups[14], Group::Department);
    assert_eq!(
        view.rows[0].sub.as_deref(),
        Some("Kisima WASH: Turkana boreholes")
    );
    assert_eq!(view.rows[8].sub, None);
    assert_eq!(Group::Grant.title(), "Grant budgets");
    assert_eq!(Group::Running.title(), "Unrestricted running costs");
    assert_eq!(Group::Department.title(), "Department allocations");

    let field_ops = &view.rows[14];
    assert_eq!(field_ops.row.label, "All expenses \u{b7} Field Operations");
    assert_eq!(field_ops.row.status, WarningLevel::Over);
    assert_eq!(field_ops.status_line(), "Over budget \u{b7} 111% used");
    assert_eq!(field_ops.used_cell(), "111%");
    assert_eq!(field_ops.variance_label(), "Over by");
    assert_eq!(view.rows[0].status_line(), "Within budget \u{b7} 74% used");
    assert_eq!(view.rows[0].variance_label(), "Left");
}

#[test]
fn the_budget_totals_and_figures() {
    let books = build_demo_ledger().unwrap();
    let view = budget_view(&books, &filter(), &as_of()).unwrap();
    let budget: i64 = [
        340_000_000,
        30_000_000,
        5_000_000,
        240_000_000,
        80_000_000,
        520_000_000,
        80_000_000,
        900_000_000,
        1_740_000_000,
        30_000_000,
        60_000_000,
        48_000_000,
        18_000_000,
        120_000_000,
        180_000_000,
    ]
    .iter()
    .sum();
    let actual: i64 = [
        250_000_000,
        120_000_000,
        75_000_000,
        145_000_000,
        2_850_000,
        11_500_000,
        8_180_000,
        38_000_000,
        199_800_000,
    ]
    .iter()
    .sum();
    assert_eq!(view.totals.budget, budget);
    assert_eq!(view.totals.actual, actual);
    assert_eq!(view.totals.variance, budget - actual);
    assert_eq!(view.totals.warning, 0);
    assert_eq!(view.totals.over, 1);
    assert_eq!(view.totals.used_percent(), 19);
    let figures = view.figures("KES");
    let figures: Vec<(&str, &str)> = figures
        .iter()
        .map(|f| (f.value.as_str(), f.label.as_str()))
        .collect();
    assert_eq!(
        figures,
        [
            ("43.9M", "Budgeted, KES"),
            ("8.5M", "Spent"),
            ("0", "Lines 80% used"),
            ("1", "Lines over budget")
        ]
    );
}

#[test]
fn the_table_has_a_heading_per_group_a_row_per_line_and_totals() {
    let books = build_demo_ledger().unwrap();
    let view = budget_view(&books, &filter(), &as_of()).unwrap();
    let table = view.table();
    assert_eq!(table.first_column, "Line");
    assert_eq!(table.columns, ["Budget", "Actual", "Variance", "Used"]);
    let kinds: Vec<RowKind> = table.rows.iter().map(|r| r.kind).collect();
    assert_eq!(kinds.iter().filter(|k| **k == RowKind::Heading).count(), 3);
    assert_eq!(kinds.iter().filter(|k| **k == RowKind::Line).count(), 15);
    assert_eq!(kinds.last(), Some(&RowKind::Total));
    assert_eq!(table.rows[0].label, "Grant budgets");
    let first = &table.rows[1];
    assert_eq!(
        first.label,
        "5100 Programme: Water and sanitation \u{b7} Kisima WASH: Turkana boreholes"
    );
    assert_eq!(
        first.cells,
        [
            Cell::Money(340_000_000),
            Cell::Money(250_000_000),
            Cell::Money(90_000_000),
            Cell::Text("74%".to_owned())
        ]
    );
    assert_eq!(
        first.detail.as_deref(),
        Some("Within budget \u{b7} 74% used")
    );
    let total = table.rows.last().unwrap();
    assert_eq!(total.label, "Total");
    assert_eq!(
        total.cells,
        [
            Cell::Money(view.totals.budget),
            Cell::Money(view.totals.actual),
            Cell::Money(view.totals.variance),
            Cell::Text("19%".to_owned())
        ]
    );
}

#[test]
fn zero_budgets_read_zero_or_infinity_as_the_web_shows_them() {
    let mut backend = backend();
    // A zero line on communication fees, where nothing is spent: used 0.
    let mut quiet = BudgetLineForm::new(BudgetId::from("budget-2026"));
    quiet.account = Some(account_id("7400"));
    quiet.amount = "0".to_owned();
    backend.submit(&demo(), quiet.command().unwrap()).unwrap();
    // A zero line on vehicle running, where 115,000.00 is spent: used Infinity.
    let mut spent = BudgetLineForm::new(BudgetId::from("budget-2026"));
    spent.account = Some(account_id("7200"));
    spent.amount = "0".to_owned();
    backend.submit(&demo(), spent.command().unwrap()).unwrap();

    let view = budget_view(&books(&backend), &filter(), &as_of()).unwrap();
    let find = |code: &str| -> &BudgetRow {
        view.rows
            .iter()
            .filter(|r| r.line.account_id == Some(account_id(code)) && r.line.amount == 0)
            .last()
            .unwrap()
    };
    let quiet = find("7400");
    assert!(quiet.row.used.abs() < f64::EPSILON);
    assert_eq!(quiet.row.status, WarningLevel::Within);
    assert_eq!(quiet.status_line(), "Within budget \u{b7} 0% used");
    assert_eq!(quiet.used_cell(), "0%");

    let spent = find("7200");
    assert!(spent.row.used.is_infinite());
    assert_eq!(spent.row.status, WarningLevel::Over);
    assert_eq!(spent.row.actual, 11_500_000);
    assert_eq!(
        spent.status_line(),
        "Over budget \u{b7} spent with no budget"
    );
    assert_eq!(spent.used_cell(), "\u{2013}");
    assert_eq!(spent.variance_label(), "Over by");
    assert_eq!(view.totals.over, 2);
}

#[test]
fn the_filters_narrow_the_lines() {
    let books = build_demo_ledger().unwrap();
    let by_grant = budget_view(
        &books,
        &BudgetFilter {
            grant: Some(grant("grant-paef-education")),
            ..filter()
        },
        &as_of(),
    )
    .unwrap();
    let ids: Vec<&str> = by_grant.rows.iter().map(|r| r.line.id.as_str()).collect();
    assert_eq!(ids, ["demo-003d", "demo-003e"]);
    let by_dimension = budget_view(
        &books,
        &BudgetFilter {
            dimension: Some(DimensionId::from("dim-a16")),
            ..filter()
        },
        &as_of(),
    )
    .unwrap();
    assert_eq!(by_dimension.rows.len(), 1);
    let cash = budget_view(
        &books,
        &BudgetFilter {
            basis: Basis::Cash,
            ..filter()
        },
        &as_of(),
    )
    .unwrap();
    assert_eq!(cash.rows.len(), 15);
}

#[test]
fn books_without_a_budget_have_no_budget_view() {
    let backend = backend();
    let empty = &backend.organisations()[1].id;
    let books = backend.state(empty).unwrap();
    assert!(budget_view(&books, &filter(), &as_of()).is_none());
}

#[test]
fn the_chart_has_a_budget_and_an_actual_bar_per_line() {
    let books = build_demo_ledger().unwrap();
    let view = budget_view(&books, &filter(), &as_of()).unwrap();
    let bars = view.bars();
    assert_eq!(bars.len(), 30);
    assert_eq!(bars[0].kind, BarKind::Budget);
    assert_eq!(bars[1].kind, BarKind::Actual);
    assert!((bars[0].major - 3_400_000.0).abs() < 1e-9);
    assert!((bars[1].major - 2_500_000.0).abs() < 1e-9);
    assert_eq!(bars[29].status, WarningLevel::Over);
    let mut bands: Vec<&str> = bars.iter().map(|b| b.band.as_str()).collect();
    bands.sort_unstable();
    bands.dedup();
    assert_eq!(bands.len(), 30, "band labels are unique");
}

// Forms and commands.

#[test]
fn the_grant_form_checks_its_fields() {
    let books = build_demo_ledger().unwrap();
    let form = NewGrantForm::new(&books);
    assert_eq!(form.currency, "KES");
    let errors = form.command().unwrap_err();
    assert_eq!(errors.name.as_deref(), Some("Enter the grant name."));
    assert_eq!(errors.donor.as_deref(), Some("Enter the donor's name."));
    assert_eq!(
        errors.fund.as_deref(),
        Some("Choose the fund the grant sits in.")
    );
    assert_eq!(errors.award.as_deref(), Some("Enter the award."));
    assert_eq!(errors.start.as_deref(), Some("Enter a date as YYYY-MM-DD."));
    assert_eq!(errors.end.as_deref(), Some("Enter a date as YYYY-MM-DD."));

    let mut form = NewGrantForm::new(&books);
    form.award = "12.345".to_owned();
    assert_eq!(
        form.command().unwrap_err().award.as_deref(),
        Some("Enter an amount, like 250000.")
    );
}

#[test]
fn the_grant_form_builds_grant_add() {
    let books = build_demo_ledger().unwrap();
    let mut form = NewGrantForm::new(&books);
    form.name = "  Rift Valley nutrition ".to_owned();
    form.donor = "Example Trust".to_owned();
    form.fund = Some(FundId::from("fund-maternal-health"));
    form.award = "1,250,000.50".to_owned();
    form.start = "2026-03-01".to_owned();
    form.end = "2027-02-28".to_owned();
    form.terms = "Quarterly reports.".to_owned();
    let LedgerCommand::GrantAdd { input } = form.command().unwrap() else {
        panic!("grant.add");
    };
    assert_eq!(input.id, None);
    assert_eq!(input.name, "Rift Valley nutrition");
    assert_eq!(input.donor_name, "Example Trust");
    assert_eq!(input.fund_id.as_str(), "fund-maternal-health");
    assert!((input.award - 125_000_050.0).abs() < f64::EPSILON);
    assert_eq!(input.currency, "KES");
    assert_eq!(input.start_date.as_str(), "2026-03-01");
    assert_eq!(input.end_date.as_str(), "2027-02-28");
    assert_eq!(input.terms, "Quarterly reports.");
}

#[test]
fn engine_refusals_of_a_grant_show_byte_equal() {
    let mut backend = backend();
    let mut form = NewGrantForm::new(&books(&backend));
    form.name = "Backwards".to_owned();
    form.donor = "Donor".to_owned();
    form.fund = Some(FundId::from(UNRESTRICTED_FUND_ID));
    form.award = "100".to_owned();
    form.start = "2026-06-01".to_owned();
    form.end = "2026-01-01".to_owned();
    let err = backend
        .submit(&demo(), form.command().unwrap())
        .unwrap_err();
    assert_eq!(
        Failure::new("Not added", &err).messages,
        ["A grant cannot end before it starts."]
    );

    form.end = "2026-12-31".to_owned();
    form.award = "0".to_owned();
    let err = backend
        .submit(&demo(), form.command().unwrap())
        .unwrap_err();
    assert_eq!(
        Failure::new("Not added", &err).messages,
        ["The award must be positive."]
    );

    form.award = "100".to_owned();
    form.fund = Some(FundId::from("fund-nowhere"));
    let err = backend
        .submit(&demo(), form.command().unwrap())
        .unwrap_err();
    assert_eq!(
        Failure::new("Not added", &err).messages,
        ["Fund not found."]
    );
}

#[test]
fn an_added_grant_lists_with_nothing_spent() {
    let mut backend = backend();
    let id = small_grant(&mut backend);
    let row = row_of(&backend, &id);
    assert_eq!(row.summary.award, 100_000);
    assert_eq!(row.summary.spent, 0);
    assert_eq!(
        row.status_line(),
        "Within award \u{b7} 0% spent or committed"
    );
}

#[test]
fn the_line_form_checks_the_amount_and_builds_budget_line_set() {
    let mut form = BudgetLineForm::new(BudgetId::from("budget-2026"));
    assert_eq!(form.command().unwrap_err(), "Enter the budgeted amount.");
    form.amount = "-5".to_owned();
    assert_eq!(
        form.command().unwrap_err(),
        "Enter an amount of zero or more, like 250000."
    );
    form.amount = "abc".to_owned();
    assert_eq!(
        form.command().unwrap_err(),
        "Enter an amount of zero or more, like 250000."
    );
    form.amount = "250,000".to_owned();
    form.account = Some(account_id("5700"));
    form.grant = Some(grant("grant-kisima-wash"));
    let LedgerCommand::BudgetLineSet { input } = form.command().unwrap() else {
        panic!("budgetLine.set");
    };
    assert_eq!(input.id, None);
    assert_eq!(input.budget_id.as_str(), "budget-2026");
    assert_eq!(input.account_id, Nullish::Value(account_id("5700")));
    assert_eq!(input.fund_id, Nullish::Null);
    assert_eq!(input.grant_id, Nullish::Value(grant("grant-kisima-wash")));
    assert_eq!(input.dimension_id, Nullish::Null);
    assert_eq!(input.period_id, Nullish::Null);
    assert!((input.amount - 25_000_000.0).abs() < f64::EPSILON);
}

#[test]
fn choosing_a_fund_drops_a_grant_of_another_fund() {
    let books = build_demo_ledger().unwrap();
    let mut form = BudgetLineForm::new(BudgetId::from("budget-2026"));
    form.grant = Some(grant("grant-kisima-wash"));
    form.set_fund(Some(FundId::from("fund-wash")), &books);
    assert_eq!(form.grant, Some(grant("grant-kisima-wash")));
    form.set_fund(Some(FundId::from("fund-education")), &books);
    assert_eq!(form.grant, None);
}

#[test]
fn only_active_posting_expense_accounts_take_a_line() {
    let books = build_demo_ledger().unwrap();
    let choices = expense_choices(&books);
    assert!(!choices.is_empty());
    let codes: Vec<&str> = choices
        .iter()
        .map(|(_, label)| label.split(' ').next().unwrap())
        .collect();
    let mut sorted = codes.clone();
    sorted.sort_unstable();
    assert_eq!(codes, sorted);
    for (id, label) in &choices {
        let a = books.account(id).unwrap();
        assert_eq!(a.account_type, ledger_core::types::AccountType::Expense);
        assert!(!a.is_group && a.is_active);
        assert_eq!(*label, format!("{} \u{b7} {}", a.code, a.name));
    }
}

#[test]
fn engine_refusals_of_a_line_show_byte_equal() {
    let mut backend = backend();
    let mut form = BudgetLineForm::new(BudgetId::from("budget-2026"));
    form.account = Some(AccountId::from("acc-1110"));
    form.amount = "100".to_owned();
    let err = backend
        .submit(&demo(), form.command().unwrap())
        .unwrap_err();
    assert_eq!(
        Failure::new("Line not added", &err).messages,
        ["Budget lines are set on expense accounts."]
    );
    let mut form = BudgetLineForm::new(BudgetId::from("budget-1999"));
    form.amount = "100".to_owned();
    let err = backend
        .submit(&demo(), form.command().unwrap())
        .unwrap_err();
    assert_eq!(
        Failure::new("Line not added", &err).messages,
        ["Budget not found."]
    );
}

#[test]
fn changing_a_line_keeps_its_fields_and_sets_the_amount() {
    let mut backend = backend();
    let view = budget_view(&books(&backend), &filter(), &as_of()).unwrap();
    let row = &view.rows[0];
    let mut edit = LineEdit::new(row);
    assert_eq!(edit.value, "3400000.00");
    edit.value = "-1".to_owned();
    assert_eq!(
        edit.command("KES").unwrap_err(),
        "Enter an amount of zero or more, like 250000."
    );
    edit.value = "3,500,000".to_owned();
    let (command, saved) = edit.command("KES").unwrap();
    assert_eq!(
        saved,
        "Budget for 5100 Programme: Water and sanitation \u{b7} Kisima WASH: Turkana boreholes \
         set to KES 3.5M."
    );
    let LedgerCommand::BudgetLineSet { input } = &command else {
        panic!("budgetLine.set");
    };
    assert_eq!(input.id.as_ref().map(|i| i.as_str()), Some("demo-0038"));
    assert_eq!(input.grant_id, Nullish::Value(grant("grant-kisima-wash")));
    assert_eq!(input.fund_id, Nullish::Null);
    backend.submit(&demo(), command).unwrap();
    let after = budget_view(&books(&backend), &filter(), &as_of()).unwrap();
    assert_eq!(after.rows.len(), 15);
    assert_eq!(after.rows[0].row.budget, 350_000_000);
    assert_eq!(after.rows[0].row.actual, 250_000_000);
    assert_eq!(
        after.rows[0].line.grant_id,
        Some(grant("grant-kisima-wash"))
    );
}
