//! View models of the Spend entry's accounts-payable half: vendors, the bills list with its
//! status filter, the new-bill form, the bill detail with its status-to-actions mapping, its
//! journal entries and payments, and the payment form. Plain structs, no GPUI; the panel in
//! `panels::bills` only draws them.
//!
//! Behaviour mirrors the web screens `flowtrace/src/routes/_authenticated/spend/bills.index.tsx`,
//! `bills.new.tsx`, `bills.$id.tsx` and `-shared.tsx`; the rules themselves stay in
//! `ledger-core` (`add_vendor`, `create_bill`, `approve_bill`, `post_bill`, `pay_bill`,
//! `outstanding`), whose messages are shown as they come. The posting and payment entries are
//! read back from the books, never recomputed here.

use ledger_core::LedgerCommand;
use ledger_core::Minor;
use ledger_core::coa_template::UNRESTRICTED_FUND_ID;
use ledger_core::engine::{NewBill, NewCodedLine, NewVendor};
use ledger_core::money::parse_money;
use ledger_core::types::{
    AccountId, AccountType, Actor, AppliesTo, Bill, BillId, BillStatus, DimensionId, EntryId,
    FundId, FundRestriction, GrantId, IsoDate, LedgerState, Nullish, VendorId,
};

use crate::accounts::{ACCOUNT_TYPES, format_date, format_minor};
use crate::general_ledger::EntryDetail;

/// The en dash of an empty cell (`money.tsx:12`).
const DASH: &str = "\u{2013}";

/// The middle dot between parts of a label.
const DOT: &str = " \u{b7} ";

/// `BILL_STATUS_LABEL` (`-shared.tsx:60-66`).
#[must_use]
pub fn status_label(status: BillStatus) -> &'static str {
    match status {
        BillStatus::Draft => "Draft",
        BillStatus::Approved => "Approved",
        BillStatus::Posted => "Posted, to pay",
        BillStatus::Paid => "Paid",
        BillStatus::Void => "Void",
    }
}

/// The look of a status tag (`-shared.tsx:68-85`, `BILL_TONE`): dashed for drafts, outlined
/// while open, filled ink once booked, filled blue when settled, dotted red when void.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tone {
    /// A draft.
    Quiet,
    /// Approved, not yet in the books.
    Open,
    /// Posted, money owed.
    Booked,
    /// Paid.
    Done,
    /// Void.
    Rejected,
}

/// `BILL_TONE` (`-shared.tsx:79-85`).
#[must_use]
pub fn status_tone(status: BillStatus) -> Tone {
    match status {
        BillStatus::Draft => Tone::Quiet,
        BillStatus::Approved => Tone::Open,
        BillStatus::Posted => Tone::Booked,
        BillStatus::Paid => Tone::Done,
        BillStatus::Void => Tone::Rejected,
    }
}

/// One choice of the list's status filter (`bills.index.tsx:15-22`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BillFilter {
    /// Every bill.
    All,
    /// Bills in one status.
    Status(BillStatus),
}

/// The filters in the order the list shows them (`bills.index.tsx:15-22`).
pub const FILTERS: [BillFilter; 6] = [
    BillFilter::All,
    BillFilter::Status(BillStatus::Draft),
    BillFilter::Status(BillStatus::Approved),
    BillFilter::Status(BillStatus::Posted),
    BillFilter::Status(BillStatus::Paid),
    BillFilter::Status(BillStatus::Void),
];

impl BillFilter {
    /// The filter's label: `All`, `Draft`, `Approved`, `To pay`, `Paid`, `Void`.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::All => "All",
            Self::Status(BillStatus::Posted) => "To pay",
            Self::Status(s) => status_label(s),
        }
    }

    /// Whether a bill in `status` shows under this filter.
    #[must_use]
    pub fn matches(self, status: BillStatus) -> bool {
        match self {
            Self::All => true,
            Self::Status(s) => s == status,
        }
    }
}

/// Days since 1970-01-01 of a valid date (Howard Hinnant's `days_from_civil`,
/// <https://howardhinnant.github.io/date_algorithms.html>).
fn day_number(date: &IsoDate) -> i64 {
    let s = date.as_str();
    let part = |r: std::ops::Range<usize>| s.get(r).and_then(|p| p.parse::<i64>().ok());
    let (y, m, d) = (
        part(0..4).unwrap_or(1970),
        part(5..7).unwrap_or(1),
        part(8..10).unwrap_or(1),
    );
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let doy = (153 * (m + if m > 2 { -3 } else { 9 }) + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// `daysBetween` (`-shared.tsx:130-134`): whole days from `a` to `b`, negative when `b` is
/// earlier.
#[must_use]
pub fn days_between(a: &IsoDate, b: &IsoDate) -> i64 {
    day_number(b) - day_number(a)
}

/// One row of the bills list (`bills.index.tsx:33-55`, `rowsFor`).
#[derive(Debug, Clone, PartialEq)]
pub struct BillRow {
    /// The bill.
    pub id: BillId,
    /// The vendor's name, or `Vendor` when the vendor is gone.
    pub vendor: String,
    /// The vendor's bill number.
    pub number: String,
    /// Bill date.
    pub bill_date: IsoDate,
    /// Due date.
    pub due_date: IsoDate,
    /// Bill currency.
    pub currency: String,
    /// Sum of the coded lines, in the bill's currency.
    pub total: Minor,
    /// What is still owed once posted (`ledger.outstanding`); `None` before posting.
    pub outstanding: Option<Minor>,
    /// Days past the due date of a posted bill with money owed, otherwise 0.
    pub overdue_days: i64,
    /// Status.
    pub status: BillStatus,
}

impl BillRow {
    /// The outstanding cell: `Not posted`, or the amount.
    #[must_use]
    pub fn outstanding_text(&self) -> String {
        self.outstanding
            .map_or_else(|| "Not posted".to_owned(), format_minor)
    }
}

/// `OverdueFlag` (`-shared.tsx:121-128`): `Overdue 1 day`, `Overdue 5 days`.
#[must_use]
pub fn overdue_text(days: i64) -> String {
    format!("Overdue {days} {}", if days == 1 { "day" } else { "days" })
}

/// The outstanding amount of `bill`, as the web reads it: only once posted or paid
/// (`bills.index.tsx:43`, `bills.$id.tsx:83`).
#[must_use]
pub fn outstanding_of(state: &LedgerState, bill: &Bill) -> Option<Minor> {
    matches!(bill.status, BillStatus::Posted | BillStatus::Paid)
        .then(|| state.outstanding(&AppliesTo::Bill(bill.id.clone())).ok())
        .flatten()
}

/// Days late of a posted bill with money owed at `as_of` (`bills.index.tsx:44`).
fn overdue_days(bill: &Bill, outstanding: Option<Minor>, as_of: &IsoDate) -> i64 {
    if bill.status == BillStatus::Posted && outstanding.unwrap_or(0) > 0 {
        days_between(&bill.due_date, as_of).max(0)
    } else {
        0
    }
}

fn vendor_name(state: &LedgerState, id: &VendorId, missing: &str) -> String {
    state
        .vendors
        .iter()
        .find(|v| &v.id == id)
        .map_or_else(|| missing.to_owned(), |v| v.name.clone())
}

/// `docTotal` (`-shared.tsx:136`).
fn doc_total(bill: &Bill) -> Minor {
    bill.lines.iter().map(|l| l.amount).sum()
}

/// `rowsFor` (`bills.index.tsx:41-55`): every bill in the books' order, with its total, what is
/// still owed (posted and paid bills only) and how late it is at `as_of`.
#[must_use]
pub fn bill_rows(state: &LedgerState, as_of: &IsoDate) -> Vec<BillRow> {
    state
        .bills
        .iter()
        .map(|bill| {
            let outstanding = outstanding_of(state, bill);
            BillRow {
                id: bill.id.clone(),
                vendor: vendor_name(state, &bill.vendor_id, "Vendor"),
                number: bill.number.clone(),
                bill_date: bill.bill_date.clone(),
                due_date: bill.due_date.clone(),
                currency: bill.currency.clone(),
                total: doc_total(bill),
                outstanding,
                overdue_days: overdue_days(bill, outstanding, as_of),
                status: bill.status,
            }
        })
        .collect()
}

/// The rows `filter` shows, most overdue first, then the newest bill date first
/// (`bills.index.tsx:66-73`; dates compare as strings, as `localeCompare` does for ISO dates).
#[must_use]
pub fn filtered(rows: &[BillRow], filter: BillFilter) -> Vec<BillRow> {
    let mut out: Vec<BillRow> = rows
        .iter()
        .filter(|r| filter.matches(r.status))
        .cloned()
        .collect();
    out.sort_by(|a, b| {
        b.overdue_days
            .cmp(&a.overdue_days)
            .then_with(|| b.bill_date.as_str().cmp(a.bill_date.as_str()))
    });
    out
}

/// The count beside a filter (`bills.index.tsx:61-65`).
#[must_use]
pub fn filter_count(rows: &[BillRow], filter: BillFilter) -> usize {
    rows.iter().filter(|r| filter.matches(r.status)).count()
}

/// What the list says when a filter shows nothing (`bills.index.tsx:136-146`).
#[must_use]
pub fn no_match_text(filter: BillFilter) -> String {
    match filter {
        BillFilter::All => "No bills are here.".to_owned(),
        BillFilter::Status(s) => format!("No bills are {}.", status_label(s).to_lowercase()),
    }
}

/// One row of the vendors list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VendorRow {
    /// The vendor.
    pub id: VendorId,
    /// Display name.
    pub name: String,
    /// Payment details (paybill, till or bank account), the only contact the books keep.
    pub payment_details: Option<String>,
    /// The flow-map node id when the vendor stands for one (the books keep no node names).
    pub flow_node: Option<String>,
    /// How many bills the vendor has.
    pub bills: usize,
    /// What is still owed on the vendor's posted bills, in base currency at each bill's rate.
    pub outstanding: Minor,
}

/// A bill's owed amount in base currency at its booked rate (`spend/index.tsx:859-860`,
/// `inBase`: `Math.round(amount * rate)`).
fn in_base(state: &LedgerState, bill: &Bill, amount: Minor) -> Minor {
    if bill.currency == state.settings.base_currency {
        return amount;
    }
    #[expect(clippy::cast_precision_loss, reason = "minor units stay below 2^53")]
    let x = ledger_core::money::js_round(amount as f64 * bill.fx_rate);
    #[expect(clippy::cast_possible_truncation, reason = "a whole number of minor units")]
    let minor = x as Minor;
    minor
}

/// The vendors in name order (`bills.new.tsx`, `vendorOptions`), each with its bill count and
/// what is still owed on its posted bills.
#[must_use]
pub fn vendor_rows(state: &LedgerState) -> Vec<VendorRow> {
    let mut rows: Vec<VendorRow> = state
        .vendors
        .iter()
        .map(|v| {
            let bills: Vec<&Bill> = state.bills.iter().filter(|b| b.vendor_id == v.id).collect();
            let outstanding = bills
                .iter()
                .filter(|b| b.status == BillStatus::Posted)
                .map(|b| in_base(state, b, outstanding_of(state, b).unwrap_or(0)))
                .sum();
            let flow_node = v.node_id.as_ref().map(|id| id.as_str().to_owned());
            VendorRow {
                id: v.id.clone(),
                name: v.name.clone(),
                payment_details: v.payment_details.clone(),
                flow_node,
                bills: bills.len(),
                outstanding,
            }
        })
        .collect();
    rows.sort_by(|a, b| a.name.cmp(&b.name));
    rows
}

/// The vendors section meta: `5 suppliers · 39,800.00 owed`.
#[must_use]
pub fn vendors_meta(rows: &[VendorRow]) -> String {
    let owed: Minor = rows.iter().map(|r| r.outstanding).sum();
    let count = match rows.len() {
        1 => "1 supplier".to_owned(),
        n => format!("{n} suppliers"),
    };
    if owed == 0 {
        format!("{count}{DOT}nothing owed")
    } else {
        format!("{count}{DOT}{} owed", format_minor(owed))
    }
}

/// The add-a-vendor form (`bills.new.tsx`, the "Add a new supplier" fields).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NewVendorForm {
    /// Supplier name.
    pub name: String,
    /// Paybill, till or bank account.
    pub payment_details: String,
}

impl NewVendorForm {
    /// The `vendor.add` command (`bills.new.tsx`, `save`: name trimmed, empty details as
    /// `null`), or the name field's error.
    ///
    /// # Errors
    ///
    /// The message shown under the name field when it is blank (`bills.new.tsx`, `check`).
    pub fn command(&self) -> Result<LedgerCommand, String> {
        let name = self.name.trim();
        if name.is_empty() {
            return Err("Enter the supplier\u{2019}s name as it appears on the bill.".to_owned());
        }
        let details = self.payment_details.trim();
        Ok(LedgerCommand::VendorAdd {
            input: NewVendor {
                id: None,
                name: name.to_owned(),
                node_id: None,
                payment_details: (!details.is_empty()).then(|| details.to_owned()),
            },
        })
    }
}

/// The month names of the date field's messages (`date-field.tsx:7-20`).
const MONTHS: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];

fn days_in_month(year: i64, month: i64) -> i64 {
    match month {
        2 if (year % 4 == 0 && year % 100 != 0) || year % 400 == 0 => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}

/// `parseDayMonthYear` (`flowtrace/src/components/ui/date-field.tsx:50-80`): `Ok(None)` for an
/// empty field, `Ok(Some(date))` for a valid `dd/mm/yyyy`, otherwise the plain-words problem.
///
/// # Errors
///
/// The message the web field shows for a malformed or impossible date.
pub fn parse_day_month_year(text: &str) -> Result<Option<IsoDate>, String> {
    let t = text.trim();
    if t.is_empty() {
        return Ok(None);
    }
    let malformed = || "Enter the date as day/month/year, for example 18/01/2026.".to_owned();
    let parts: Vec<&str> = t.split(['/', '.', '-']).collect();
    let digits = |p: &str| !p.is_empty() && p.bytes().all(|b| b.is_ascii_digit());
    let [d, m, y] = parts.as_slice() else {
        return Err(malformed());
    };
    if !(digits(d) && digits(m) && digits(y) && d.len() <= 2 && m.len() <= 2) {
        return Err(malformed());
    }
    if y.len() != 4 {
        return Err("Write the year in full, for example 2026.".to_owned());
    }
    let (day, month, year): (i64, i64, i64) = (
        d.parse().map_err(|_| malformed())?,
        m.parse().map_err(|_| malformed())?,
        y.parse().map_err(|_| malformed())?,
    );
    if !(1..=12).contains(&month) {
        return Err(format!(
            "There is no month {month}. Put the day first, then the month: dd/mm/yyyy."
        ));
    }
    let days = days_in_month(year, month);
    if day < 1 {
        return Err("There is no day 0. Enter a day from 1.".to_owned());
    }
    if day > days {
        let name = usize::try_from(month - 1)
            .ok()
            .and_then(|i| MONTHS.get(i))
            .copied()
            .unwrap_or_default();
        return Err(format!("{name} {year} has only {days} days."));
    }
    IsoDate::parse(&format!("{year:04}-{month:02}-{day:02}"))
        .map(Some)
        .map_err(|_| malformed())
}

/// `isoToDayMonthYear` (`date-field.tsx:36-39`): `2026-01-18` to `18/01/2026`.
#[must_use]
pub fn iso_to_day_month_year(date: &IsoDate) -> String {
    let s = date.as_str();
    match (s.get(0..4), s.get(5..7), s.get(8..10)) {
        (Some(y), Some(m), Some(d)) => format!("{d}/{m}/{y}"),
        _ => String::new(),
    }
}

/// `addDays` (`bills.new.tsx:49-53`).
#[must_use]
pub fn add_days(date: &IsoDate, n: i64) -> IsoDate {
    crate::statements::date_from_days(day_number(date) + n)
}

/// `CURRENCIES` (`bills.new.tsx:27-34`): code and name.
pub const CURRENCIES: [(&str, &str); 6] = [
    ("KES", "Kenyan shilling"),
    ("USD", "US dollar"),
    ("EUR", "Euro"),
    ("GBP", "British pound"),
    ("UGX", "Ugandan shilling"),
    ("TZS", "Tanzanian shilling"),
];

/// One coded line being entered (`LineDraft`, `-shared.tsx:288-299`; bills use one dimension).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LineDraft {
    /// What it was for.
    pub description: String,
    /// The amount as typed.
    pub amount: String,
    /// Kind of cost.
    pub account_id: Option<AccountId>,
    /// Fund.
    pub fund_id: Option<FundId>,
    /// Grant, if any.
    pub grant_id: Option<GrantId>,
    /// Project, department or place, if any.
    pub dimension_id: Option<DimensionId>,
}

impl LineDraft {
    /// `newLine(state)` (`-shared.tsx:301-320`): empty, on the unrestricted fund when the books
    /// have it, else the first fund.
    #[must_use]
    pub fn new(state: &LedgerState) -> Self {
        let fund = state
            .funds
            .iter()
            .find(|f| f.id.as_str() == UNRESTRICTED_FUND_ID)
            .or_else(|| state.funds.first())
            .map(|f| f.id.clone());
        Self {
            fund_id: fund,
            ..Self::default()
        }
    }

    /// The line "Add another line" adds after this one: same account, fund, grant and
    /// dimension, empty description and amount (`-shared.tsx:551-568`).
    #[must_use]
    pub fn following(&self) -> Self {
        Self {
            description: String::new(),
            amount: String::new(),
            ..self.clone()
        }
    }

    /// `withFund` (`-shared.tsx:373-378`): changing the fund drops a grant of another fund and
    /// picks the fund's only grant.
    pub fn set_fund(&mut self, state: &LedgerState, fund: FundId) {
        let grants: Vec<&GrantId> = state
            .grants
            .iter()
            .filter(|g| g.fund_id == fund)
            .map(|g| &g.id)
            .collect();
        let keep = self.grant_id.as_ref().is_some_and(|g| grants.contains(&g));
        if !keep {
            self.grant_id = match grants.as_slice() {
                [only] => Some((*only).clone()),
                _ => None,
            };
        }
        self.fund_id = Some(fund);
    }
}

/// `code · name` of an account.
fn account_label(code: &str, name: &str) -> String {
    format!("{code}{DOT}{name}")
}

/// `AccountSelect` (`pickers.tsx:25-44`): active posting accounts of `types`, by type in
/// `TYPE_LABEL` order, then by code.
fn accounts_of(state: &LedgerState, types: &[AccountType], cash_only: bool) -> Vec<(AccountId, String)> {
    let mut out = Vec::new();
    for t in ACCOUNT_TYPES.into_iter().filter(|t| types.contains(t)) {
        let mut of_type: Vec<_> = state
            .accounts
            .iter()
            .filter(|a| a.account_type == t && !a.is_group && a.is_active && (!cash_only || a.is_cash))
            .collect();
        of_type.sort_by(|a, b| a.code.cmp(&b.code));
        out.extend(of_type.into_iter().map(|a| (a.id.clone(), account_label(&a.code, &a.name))));
    }
    out
}

/// `AccountSelect` with `types={['expense', 'asset']}` (`pickers.tsx:25-44`,
/// `bills.new.tsx`): active posting accounts of those types, by type in `TYPE_LABEL` order, then
/// by code; labels `code · name`.
#[must_use]
pub fn line_accounts(state: &LedgerState) -> Vec<(AccountId, String)> {
    accounts_of(state, &[AccountType::Expense, AccountType::Asset], false)
}

/// `AccountSelect cashOnly` (`pickers.tsx:25-44`): the active cash accounts, by code.
#[must_use]
pub fn cash_accounts(state: &LedgerState) -> Vec<(AccountId, String)> {
    accounts_of(state, &ACCOUNT_TYPES, true)
}

/// `GrantSelect fundId` (`pickers.tsx:57-69`): the grants on `fund`.
#[must_use]
pub fn grants_for(state: &LedgerState, fund: Option<&FundId>) -> Vec<(GrantId, String)> {
    state
        .grants
        .iter()
        .filter(|g| fund.is_none_or(|f| &g.fund_id == f))
        .map(|g| (g.id.clone(), g.name.clone()))
        .collect()
}

/// `linesTotal` (`-shared.tsx:369-371`): the lines' amounts, a line that does not parse or is
/// negative counting as zero.
#[must_use]
pub fn lines_total(lines: &[LineDraft]) -> Minor {
    lines
        .iter()
        .map(|l| parse_money(&l.amount).unwrap_or(0).max(0))
        .sum()
}

/// A field of the new-bill form (`bills.new.tsx`, `FieldName`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BillField {
    /// The supplier.
    Vendor,
    /// The bill number.
    Number,
    /// The bill date.
    BillDate,
    /// The due date.
    DueDate,
    /// The exchange rate.
    FxRate,
}

impl BillField {
    /// `FIELD_LABEL` (`bills.new.tsx:42-49`).
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Vendor => "Vendor",
            Self::Number => "Bill number",
            Self::BillDate => "Bill date",
            Self::DueDate => "Due date",
            Self::FxRate => "Exchange rate",
        }
    }
}

/// A field of a coded line (`LineField`, `-shared.tsx:322-332`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineField {
    /// What it was for.
    Description,
    /// Amount.
    Amount,
    /// Kind of cost.
    Account,
    /// Fund.
    Fund,
    /// Grant.
    Grant,
}

impl LineField {
    /// `LINE_FIELD_LABEL` (`-shared.tsx:326-332`).
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Description => "description",
            Self::Amount => "amount",
            Self::Account => "account",
            Self::Fund => "fund",
            Self::Grant => "grant",
        }
    }
}

/// What the new-bill form found wrong before sending (`bills.new.tsx`, `check`; `validateLines`,
/// `-shared.tsx:334-356`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BillErrors {
    /// Header field errors, in the form's order.
    pub fields: Vec<(BillField, String)>,
    /// Line errors: line index, field, message.
    pub lines: Vec<(usize, LineField, String)>,
}

impl BillErrors {
    /// Whether nothing is wrong.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.fields.is_empty() && self.lines.is_empty()
    }

    /// The error under `field`, if any.
    #[must_use]
    pub fn field(&self, field: BillField) -> Option<String> {
        self.fields
            .iter()
            .find(|(f, _)| *f == field)
            .map(|(_, m)| m.clone())
    }

    /// The error under `field` of line `index`, if any.
    #[must_use]
    pub fn line(&self, index: usize, field: LineField) -> Option<String> {
        self.lines
            .iter()
            .find(|(i, f, _)| *i == index && *f == field)
            .map(|(_, _, m)| m.clone())
    }

    /// `ErrorSummary`'s title (`-shared.tsx:612-614`).
    #[must_use]
    pub fn title(&self) -> &'static str {
        if self.fields.len() + self.lines.len() == 1 {
            "Fix this before saving"
        } else {
            "Fix these before saving"
        }
    }

    /// `ErrorSummary`'s items (`-shared.tsx:615-630`): `Bill number: Enter ...`,
    /// `Line 1 amount: Enter ...`.
    #[must_use]
    pub fn summary(&self) -> Vec<String> {
        self.fields
            .iter()
            .map(|(f, m)| format!("{}: {m}", f.label()))
            .chain(
                self.lines
                    .iter()
                    .map(|(i, f, m)| format!("Line {} {}: {m}", i + 1, f.label())),
            )
            .collect()
    }
}

/// `validateLines` (`-shared.tsx:334-356`).
fn line_errors(state: &LedgerState, lines: &[LineDraft]) -> Vec<(usize, LineField, String)> {
    let mut out = Vec::new();
    for (n, l) in lines.iter().enumerate() {
        let mut add = |f: LineField, m: &str| out.push((n, f, m.to_owned()));
        if l.description.trim().is_empty() {
            add(LineField::Description, "Say what this was for.");
        }
        let amount = parse_money(&l.amount);
        if l.amount.trim().is_empty() {
            add(LineField::Amount, "Enter the amount.");
        } else if let Some(a) = amount {
            if a <= 0 {
                add(LineField::Amount, "The amount must be more than zero.");
            }
        } else {
            add(LineField::Amount, "Enter a number, like 1250 or 1,250.50.");
        }
        if l.account_id.is_none() {
            add(LineField::Account, "Choose what kind of cost this is.");
        }
        match l
            .fund_id
            .as_ref()
            .and_then(|id| state.funds.iter().find(|f| &f.id == id))
        {
            None => add(LineField::Fund, "Choose the fund that pays for this."),
            Some(f) if f.restriction != FundRestriction::Unrestricted && l.grant_id.is_none() => {
                add(
                    LineField::Grant,
                    "Restricted money needs the grant it comes from.",
                );
            }
            Some(_) => {}
        }
    }
    out
}

/// The new-bill form (`bills.new.tsx`, `NewBill`), for an existing supplier.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewBillForm {
    /// The supplier.
    pub vendor: Option<VendorId>,
    /// The invoice number printed on the bill.
    pub number: String,
    /// Bill date as typed, `dd/mm/yyyy`.
    pub bill_date: String,
    /// Due date as typed.
    pub due_date: String,
    /// Bill currency.
    pub currency: String,
    /// Units of base currency per unit of `currency`, as typed; only for a foreign bill.
    pub fx_rate: String,
    /// Coded lines.
    pub lines: Vec<LineDraft>,
    /// The books' own currency.
    pub base: String,
}

impl NewBillForm {
    /// An empty form at `as_of`: dated that day, due 30 days later, in the base currency, one
    /// empty line (`bills.new.tsx:68-80`).
    #[must_use]
    pub fn new(state: &LedgerState, as_of: &IsoDate) -> Self {
        Self {
            vendor: None,
            number: String::new(),
            bill_date: iso_to_day_month_year(as_of),
            due_date: iso_to_day_month_year(&add_days(as_of, 30)),
            currency: state.settings.base_currency.clone(),
            fx_rate: String::new(),
            lines: vec![LineDraft::new(state)],
            base: state.settings.base_currency.clone(),
        }
    }

    /// Whether the bill is in another currency than the books', so it needs a rate.
    #[must_use]
    pub fn foreign(&self) -> bool {
        self.currency != self.base
    }

    /// The live total under the lines.
    #[must_use]
    pub fn total(&self) -> Minor {
        lines_total(&self.lines)
    }

    /// "Add another line": a line following the last one.
    pub fn add_line(&mut self, state: &LedgerState) {
        let next = self
            .lines
            .last()
            .map_or_else(|| LineDraft::new(state), LineDraft::following);
        self.lines.push(next);
    }

    /// Removes line `index`; the last line stays (`-shared.tsx:421-432`).
    pub fn remove_line(&mut self, index: usize) {
        if self.lines.len() > 1 && index < self.lines.len() {
            self.lines.remove(index);
        }
    }

    /// The rate as typed, commas dropped (`Number(fxRate.replace(/,/g, ''))`).
    fn rate(&self) -> Option<f64> {
        let r: f64 = self.fx_rate.replace(',', "").trim().parse().ok()?;
        (r > 0.0 && r.is_finite()).then_some(r)
    }

    /// The `bill.create` command, or what to fix first (`bills.new.tsx`, `check` and `save`).
    ///
    /// # Errors
    ///
    /// [`BillErrors`] with every field and line problem, in the form's order.
    pub fn command(&self, state: &LedgerState) -> Result<LedgerCommand, BillErrors> {
        let mut fields = Vec::new();
        let mut add = |f: BillField, m: String| fields.push((f, m));
        if self.vendor.is_none() {
            add(
                BillField::Vendor,
                "Choose the supplier, or add a new one.".to_owned(),
            );
        }
        let number = self.number.trim();
        if number.is_empty() {
            add(
                BillField::Number,
                "Enter the invoice number printed on the bill.".to_owned(),
            );
        } else if state.bills.iter().any(|b| {
            b.number.trim().to_lowercase() == number.to_lowercase()
                && Some(&b.vendor_id) == self.vendor.as_ref()
        }) {
            add(
                BillField::Number,
                "This supplier already has a bill with this number.".to_owned(),
            );
        }
        let bill_date = parse_day_month_year(&self.bill_date);
        let due_date = parse_day_month_year(&self.due_date);
        match &bill_date {
            Ok(None) => add(BillField::BillDate, "Enter the date on the bill.".to_owned()),
            Err(e) => add(BillField::BillDate, e.clone()),
            Ok(Some(_)) => {}
        }
        match (&due_date, &bill_date) {
            (Ok(None), _) => add(
                BillField::DueDate,
                "Enter when the bill must be paid.".to_owned(),
            ),
            (Err(e), _) => add(BillField::DueDate, e.clone()),
            (Ok(Some(due)), Ok(Some(billed))) if due.as_str() < billed.as_str() => add(
                BillField::DueDate,
                "The due date cannot be before the bill date.".to_owned(),
            ),
            _ => {}
        }
        if self.foreign() {
            if self.fx_rate.trim().is_empty() {
                add(
                    BillField::FxRate,
                    format!("Enter how many {} one {} is worth.", self.base, self.currency),
                );
            } else if self.rate().is_none() {
                add(
                    BillField::FxRate,
                    "Enter a positive number, like 129.45.".to_owned(),
                );
            }
        }
        let errors = BillErrors {
            fields,
            lines: line_errors(state, &self.lines),
        };
        let (Some(vendor), Ok(Some(bill_date)), Ok(Some(due_date)), true) = (
            self.vendor.clone(),
            bill_date,
            due_date,
            errors.is_empty(),
        ) else {
            return Err(errors);
        };
        // toCodedLines (`-shared.tsx:358-367`).
        let lines = self
            .lines
            .iter()
            .map(|l| {
                #[expect(clippy::cast_precision_loss, reason = "minor units stay below 2^53")]
                let amount = parse_money(&l.amount).unwrap_or(0) as f64;
                NewCodedLine {
                    description: l.description.trim().to_owned(),
                    account_id: l.account_id.clone().unwrap_or_else(|| AccountId::from("")),
                    amount,
                    fund_id: l.fund_id.clone().unwrap_or_else(|| FundId::from("")),
                    grant_id: l.grant_id.clone(),
                    dimension_ids: l.dimension_id.iter().cloned().collect(),
                }
            })
            .collect();
        Ok(LedgerCommand::BillCreate {
            input: NewBill {
                id: None,
                vendor_id: vendor,
                number: number.to_owned(),
                bill_date,
                due_date,
                currency: self.currency.clone(),
                fx_rate: if self.foreign() {
                    self.rate().unwrap_or(1.0)
                } else {
                    1.0
                },
                lines,
                proof_ids: Vec::new(),
            },
        })
    }
}

/// An action the bill detail offers for the bill's status (`bills.$id.tsx`, "What happens next").
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BillAction {
    /// `bill.approve`.
    Approve,
    /// `bill.post`.
    Post,
    /// `bill.pay`, through the payment form.
    Pay,
}

impl BillAction {
    /// The button label.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Approve => "Approve bill",
            Self::Post => "Post to the books",
            Self::Pay => "Record a payment",
        }
    }

    /// The notice after the command went through (`bills.$id.tsx`, each dialog's `setDone`).
    #[must_use]
    pub fn done(self) -> &'static str {
        match self {
            Self::Approve => "Approved. Post it to record the amount owed.",
            Self::Post => "Posted to the books.",
            Self::Pay => "Payment recorded.",
        }
    }

    /// The command of Approve or Post; Pay goes through [`PayForm::command`].
    #[must_use]
    pub fn command(self, bill: &BillId) -> Option<LedgerCommand> {
        match self {
            Self::Approve => Some(LedgerCommand::BillApprove {
                bill_id: bill.clone(),
            }),
            Self::Post => Some(LedgerCommand::BillPost {
                bill_id: bill.clone(),
                override_reason: None,
            }),
            Self::Pay => None,
        }
    }
}

/// `personName` (`-shared.tsx:143-151`): `Name (you)` for the actor, `FlowTrace` for the
/// system, otherwise the id.
fn person_name(id: &str, actor: Option<&Actor>) -> String {
    match actor {
        Some(a) if a.id.as_str() == id => format!("{} (you)", a.name),
        _ if id == "system" => "FlowTrace".to_owned(),
        _ => id.to_owned(),
    }
}

/// "What happens next" (`bills.$id.tsx`): the sentence for the bill's status and the actions
/// the actor may take. A draft is approved only by someone with approval rights who did not
/// enter it; approved bills post; posted bills take payments; paid and void bills take nothing.
#[must_use]
pub fn next_step(
    bill: &Bill,
    actor: Option<&Actor>,
    outstanding: Option<Minor>,
) -> (String, Vec<BillAction>) {
    match bill.status {
        BillStatus::Draft => {
            if actor.is_some_and(|a| a.id == bill.created_by) {
                (
                    "You entered this bill, so someone else has to approve it. The person who \
                     records a bill never approves it."
                        .to_owned(),
                    vec![],
                )
            } else if actor.is_some_and(|a| a.can_approve) {
                (
                    "Check the bill against the delivery, then approve it. Approval commits the \
                     money against its grant and budget lines."
                        .to_owned(),
                    vec![BillAction::Approve],
                )
            } else {
                (
                    "Waiting for a manager with approval rights.".to_owned(),
                    vec![],
                )
            }
        }
        BillStatus::Approved => {
            let by = bill
                .approved_by
                .as_ref()
                .map(|id| format!(" by {}", person_name(id.as_str(), actor)))
                .unwrap_or_default();
            (
                format!(
                    "Approved{by}. Posting records the costs and the amount owed to the supplier \
                     in the books."
                ),
                vec![BillAction::Post],
            )
        }
        BillStatus::Posted => (
            format!(
                "Posted. {} {} is still owed, due {}.",
                bill.currency,
                format_minor(outstanding.unwrap_or(0)),
                format_date(&bill.due_date)
            ),
            vec![BillAction::Pay],
        ),
        BillStatus::Paid => ("Paid in full. Nothing more to do.".to_owned(), vec![]),
        BillStatus::Void => ("This bill was voided.".to_owned(), vec![]),
    }
}

/// A coded line as the detail shows it (`CodedLines`, `-shared.tsx:185-282`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodedLineRow {
    /// Description, or `No description`.
    pub description: String,
    /// `code name` of the account, or `Unknown account`.
    pub account: String,
    /// Fund name, or `Fund`.
    pub fund: String,
    /// Grant name, or `No grant`.
    pub grant: String,
    /// Dimension names joined with commas, or an en dash.
    pub dimensions: String,
    /// Amount in the bill's currency.
    pub amount: Minor,
}

/// `CodedLines` (`-shared.tsx:196-241`).
fn coded_rows(state: &LedgerState, bill: &Bill) -> Vec<CodedLineRow> {
    bill.lines
        .iter()
        .map(|l| {
            let dims: Vec<String> = l
                .dimension_ids
                .iter()
                .map(|id| {
                    state
                        .dimensions
                        .iter()
                        .find(|d| &d.id == id)
                        .map_or_else(|| id.as_str().to_owned(), |d| d.name.clone())
                })
                .collect();
            CodedLineRow {
                description: if l.description.is_empty() {
                    "No description".to_owned()
                } else {
                    l.description.clone()
                },
                account: state.account(&l.account_id).map_or_else(
                    || "Unknown account".to_owned(),
                    |a| format!("{} {}", a.code, a.name),
                ),
                fund: state
                    .funds
                    .iter()
                    .find(|f| f.id == l.fund_id)
                    .map_or_else(|| "Fund".to_owned(), |f| f.name.clone()),
                grant: l
                    .grant_id
                    .as_ref()
                    .and_then(|g| state.grants.iter().find(|x| &x.id == g))
                    .map_or_else(|| "No grant".to_owned(), |g| g.name.clone()),
                dimensions: if dims.is_empty() {
                    DASH.to_owned()
                } else {
                    dims.join(", ")
                },
                amount: l.amount,
            }
        })
        .collect()
}

/// A journal entry the bill made: the accrual when posted, or a payment's entry. The lines
/// come from U4's [`EntryDetail`]; the per-account movements let tests and the trail read what
/// the entry did to the payable and the bank.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntryView {
    /// The entry as the general ledger's side panel shows it.
    pub detail: EntryDetail,
    /// Net base-currency movement per account, debit positive, in first-line order.
    pub movements: Vec<(AccountId, Minor)>,
}

impl EntryView {
    /// The entry `id`, or `None` when the books do not hold it.
    #[must_use]
    pub fn of(state: &LedgerState, id: &EntryId) -> Option<Self> {
        let detail = EntryDetail::of(state, id)?;
        let mut movements: Vec<(AccountId, Minor)> = Vec::new();
        for l in state.lines.iter().filter(|l| &l.entry_id == id) {
            match movements.iter_mut().find(|(a, _)| a == &l.account_id) {
                Some((_, m)) => *m += l.base_amount,
                None => movements.push((l.account_id.clone(), l.base_amount)),
            }
        }
        Some(Self { detail, movements })
    }

    /// Whether the debits equal the credits in base currency.
    #[must_use]
    pub fn balanced(&self) -> bool {
        self.detail.debits == self.detail.credits && self.movements.iter().map(|m| m.1).sum::<Minor>() == 0
    }

    /// The net base-currency movement on `account` in this entry (debit positive).
    #[must_use]
    pub fn movement(&self, account: &AccountId) -> Minor {
        self.movements
            .iter()
            .find(|(a, _)| a == account)
            .map_or(0, |m| m.1)
    }
}

/// One payment of the bill (`bills.$id.tsx`, Payments).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PaymentView {
    /// Payment date.
    pub date: IsoDate,
    /// Amount in the payment's currency.
    pub amount: Minor,
    /// Payment currency.
    pub currency: String,
    /// `From 1120 · Equity Bank`, with ` · <reference>` when one was given.
    pub source: String,
    /// The payment's entry.
    pub entry: Option<EntryView>,
    /// How the engine split the payment across the bill's funds: fund id, fund name, the
    /// payable cleared for that fund. Read from the entry, never recomputed.
    pub fund_split: Vec<(FundId, String, Minor)>,
}

/// One step of the bill's status trail.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrailRow {
    /// When, `4 Feb 2026`.
    pub when: String,
    /// Who: `Demo User (you)`, `FlowTrace` for the system, otherwise the actor id.
    pub who: String,
    /// What happened.
    pub what: String,
}

/// Everything the bill detail shows (`bills.$id.tsx`, `BillDetail`).
#[derive(Debug, Clone, PartialEq)]
pub struct BillDetail {
    /// The bill as stored.
    pub bill: Bill,
    /// Vendor name, or `Bill`.
    pub vendor: String,
    /// Sum of the lines.
    pub total: Minor,
    /// Still owed once posted; `None` before.
    pub outstanding: Option<Minor>,
    /// `total - (outstanding ?? total)`.
    pub paid: Minor,
    /// Days past due of a posted bill with money owed.
    pub overdue_days: i64,
    /// Days until the due date, never negative.
    pub days_until_due: i64,
    /// "What happens next" sentence.
    pub next: String,
    /// The actions the status and actor allow.
    pub actions: Vec<BillAction>,
    /// The coded lines.
    pub lines: Vec<CodedLineRow>,
    /// The accrual entry once posted.
    pub posting: Option<EntryView>,
    /// Payments, oldest first.
    pub payments: Vec<PaymentView>,
    /// The status trail: audit events on the bill, its posting and its payments.
    pub trail: Vec<TrailRow>,
}

/// The date part of a timestamp, written as the books write dates.
fn when(at: &str) -> String {
    at.get(..10)
        .and_then(|d| IsoDate::parse(d).ok())
        .map_or_else(|| at.to_owned(), |d| format_date(&d))
}

/// The detail of bill `id` as `actor` sees it at `as_of`, or `None` when the books do not hold
/// it.
#[must_use]
#[expect(
    clippy::too_many_lines,
    reason = "one pass over the bill, its entries and its payments"
)]
pub fn bill_detail(
    state: &LedgerState,
    id: &BillId,
    actor: Option<&Actor>,
    as_of: &IsoDate,
) -> Option<BillDetail> {
    let bill = state.bills.iter().find(|b| &b.id == id)?;
    let total = doc_total(bill);
    let outstanding = outstanding_of(state, bill);
    let (next, actions) = next_step(bill, actor, outstanding);
    let posting = bill
        .entry_id
        .as_ref()
        .and_then(|e| EntryView::of(state, e));
    let payable = &state.settings.system_accounts.accounts_payable;
    let mut payments: Vec<_> = state
        .payments
        .iter()
        .filter(|p| p.applies_to == AppliesTo::Bill(bill.id.clone()))
        .collect();
    payments.sort_by(|a, b| a.date.as_str().cmp(b.date.as_str()));

    let mut trail: Vec<(String, TrailRow)> = state
        .audit
        .iter()
        .filter(|e| e.subject == bill.id.as_str())
        .map(|e| {
            let what = match e.action.as_str() {
                "bill.create" => format!("Entered bill {}", e.detail),
                "bill.approve" => "Approved".to_owned(),
                other => other.to_owned(),
            };
            (
                e.at.as_str().to_owned(),
                TrailRow {
                    when: when(e.at.as_str()),
                    who: person_name(e.by.as_str(), actor),
                    what,
                },
            )
        })
        .collect();
    if let Some(entry) = bill.entry_id.as_ref().and_then(|e| state.entry(e)) {
        let at = entry.posted_at.as_ref().unwrap_or(&entry.created_at);
        trail.push((
            at.as_str().to_owned(),
            TrailRow {
                when: when(at.as_str()),
                who: person_name(
                    entry.posted_by.as_ref().unwrap_or(&entry.created_by).as_str(),
                    actor,
                ),
                what: format!("Posted as {}", entry.number),
            },
        ));
    }

    let payment_views = payments
        .iter()
        .map(|p| {
            let entry = EntryView::of(state, &p.entry_id);
            let fund_split = state
                .lines
                .iter()
                .filter(|l| l.entry_id == p.entry_id && &l.account_id == payable && l.debit > 0)
                .map(|l| {
                    let name = state
                        .funds
                        .iter()
                        .find(|f| f.id == l.fund_id)
                        .map_or_else(|| l.fund_id.as_str().to_owned(), |f| f.name.clone());
                    (l.fund_id.clone(), name, l.debit)
                })
                .collect();
            let cash = state.account(&p.cash_account_id).map_or_else(
                || "cash account".to_owned(),
                |a| account_label(&a.code, &a.name),
            );
            let source = match &p.reference {
                Some(r) if !r.is_empty() => format!("From {cash}{DOT}{r}"),
                _ => format!("From {cash}"),
            };
            if let Some(e) = state.entry(&p.entry_id) {
                let at = e.posted_at.as_ref().unwrap_or(&e.created_at);
                trail.push((
                    at.as_str().to_owned(),
                    TrailRow {
                        when: when(at.as_str()),
                        who: person_name(p.created_by.as_str(), actor),
                        what: format!("Paid {} as {}", format_minor(p.amount), e.number),
                    },
                ));
            }
            PaymentView {
                date: p.date.clone(),
                amount: p.amount,
                currency: p.currency.clone(),
                source,
                entry,
                fund_split,
            }
        })
        .collect();
    // Stable: events at the same instant keep the order above.
    trail.sort_by(|a, b| a.0.cmp(&b.0));

    Some(BillDetail {
        vendor: vendor_name(state, &bill.vendor_id, "Bill"),
        total,
        outstanding,
        paid: total - outstanding.unwrap_or(total),
        overdue_days: overdue_days(bill, outstanding, as_of),
        days_until_due: days_between(as_of, &bill.due_date).max(0),
        next,
        actions,
        lines: coded_rows(state, bill),
        posting,
        payments: payment_views,
        trail: trail.into_iter().map(|t| t.1).collect(),
        bill: bill.clone(),
    })
}

/// The payment form (`bills.$id.tsx`, the "Record a payment" dialog).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PayForm {
    /// Amount as typed; defaults to everything still owed.
    pub amount: String,
    /// Date paid as typed, `dd/mm/yyyy`.
    pub date: String,
    /// The account the money left from.
    pub cash_account: Option<AccountId>,
    /// EFT, cheque or M-Pesa code.
    pub reference: String,
}

/// What the payment form found wrong before sending.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PayErrors {
    /// Under the amount.
    pub amount: Option<String>,
    /// Under the date.
    pub date: Option<String>,
    /// Under the account.
    pub cash: Option<String>,
}

impl PayForm {
    /// The form as `openPay` fills it (`bills.$id.tsx:118-122`): the amount owed as
    /// `(outstanding / 100).toFixed(2)`, empty when nothing is owed, dated `as_of`.
    #[must_use]
    pub fn new(outstanding: Option<Minor>, as_of: &IsoDate) -> Self {
        let amount = match outstanding {
            Some(o) if o != 0 => {
                let sign = if o < 0 { "-" } else { "" };
                format!("{sign}{}.{:02}", o.unsigned_abs() / 100, o.unsigned_abs() % 100)
            }
            _ => String::new(),
        };
        Self {
            amount,
            date: iso_to_day_month_year(as_of),
            cash_account: None,
            reference: String::new(),
        }
    }

    /// The `bill.pay` command, or what to fix first (`bills.$id.tsx`, the pay dialog's
    /// `validate` and `onConfirm`).
    ///
    /// # Errors
    ///
    /// [`PayErrors`] with the message under each field that is wrong.
    pub fn command(&self, bill: &BillId, outstanding: Minor) -> Result<LedgerCommand, PayErrors> {
        let mut errors = PayErrors::default();
        let date = match parse_day_month_year(&self.date) {
            Ok(None) => {
                errors.date = Some("Enter the date the money left.".to_owned());
                None
            }
            Err(e) => {
                errors.date = Some(e);
                None
            }
            Ok(Some(d)) => Some(d),
        };
        if self.cash_account.is_none() {
            errors.cash = Some("Choose the account the money left from.".to_owned());
        }
        let amount = parse_money(&self.amount);
        if self.amount.trim().is_empty() {
            errors.amount = Some("Enter the amount paid.".to_owned());
        } else {
            match amount {
                None => {
                    errors.amount = Some("Enter a number, like 39800 or 39,800.00.".to_owned());
                }
                Some(m) if m <= 0 => {
                    errors.amount = Some("The amount must be more than zero.".to_owned());
                }
                Some(m) if m > outstanding => {
                    errors.amount = Some(format!(
                        "Only {} is outstanding.",
                        format_minor(outstanding)
                    ));
                }
                Some(_) => {}
            }
        }
        let (Some(date), Some(cash), Some(amount), PayErrors {
            amount: None,
            date: None,
            cash: None,
        }) = (date, self.cash_account.clone(), amount, &errors)
        else {
            return Err(errors);
        };
        let reference = self.reference.trim();
        #[expect(clippy::cast_precision_loss, reason = "minor units stay below 2^53")]
        let amount = amount as f64;
        Ok(LedgerCommand::BillPay {
            bill_id: bill.clone(),
            date: date.as_str().to_owned(),
            cash_account_id: cash,
            amount,
            reference: if reference.is_empty() {
                Nullish::Null
            } else {
                Nullish::Value(reference.to_owned())
            },
        })
    }
}

#[cfg(test)]
mod tests {
    #![expect(clippy::unwrap_used, reason = "tests may unwrap")]

    use super::*;
    use ledger_core::coa_template::{UNRESTRICTED_FUND_ID, account_id};
    use ledger_core::demo::{DEMO_AS_OF, build_demo_ledger, demo_actor};
    use ledger_core::types::ActorId;

    fn books() -> LedgerState {
        build_demo_ledger().unwrap()
    }

    fn as_of() -> IsoDate {
        IsoDate::parse(DEMO_AS_OF).unwrap()
    }

    fn d(s: &str) -> IsoDate {
        IsoDate::parse(s).unwrap()
    }

    fn bill_by_number<'a>(state: &'a LedgerState, number: &str) -> &'a Bill {
        state.bills.iter().find(|b| b.number == number).unwrap()
    }

    #[test]
    fn status_words_and_tones_follow_the_web() {
        let labels: Vec<_> = [
            BillStatus::Draft,
            BillStatus::Approved,
            BillStatus::Posted,
            BillStatus::Paid,
            BillStatus::Void,
        ]
        .into_iter()
        .map(|s| (status_label(s), status_tone(s)))
        .collect();
        assert_eq!(
            labels,
            [
                ("Draft", Tone::Quiet),
                ("Approved", Tone::Open),
                ("Posted, to pay", Tone::Booked),
                ("Paid", Tone::Done),
                ("Void", Tone::Rejected),
            ]
        );
        let filters: Vec<_> = FILTERS.into_iter().map(BillFilter::label).collect();
        assert_eq!(
            filters,
            ["All", "Draft", "Approved", "To pay", "Paid", "Void"]
        );
    }

    #[test]
    fn days_between_counts_calendar_days() {
        assert_eq!(days_between(&d("2026-02-09"), &d("2026-02-04")), -5);
        assert_eq!(days_between(&d("2026-01-26"), &d("2026-02-04")), 9);
        assert_eq!(days_between(&d("2025-12-31"), &d("2026-03-01")), 60);
        assert_eq!(add_days(&d("2026-02-04"), 30), d("2026-03-06"));
        assert_eq!(add_days(&d("2024-02-28"), 1), d("2024-02-29"));
    }

    #[test]
    fn the_demo_bills_list_with_totals_and_outstanding() {
        let state = books();
        let rows = bill_rows(&state, &as_of());
        assert_eq!(rows.len(), 3);
        let power = rows.iter().find(|r| r.number == "KPLC-0226-77140").unwrap();
        assert_eq!(power.vendor, "Kenya Power");
        assert_eq!(power.total, 3_980_000);
        assert_eq!(power.outstanding, Some(3_980_000));
        assert_eq!(power.status, BillStatus::Posted);
        // Due 16 Feb, as of 4 Feb: not late.
        assert_eq!(power.overdue_days, 0);
        assert_eq!(power.outstanding_text(), "39,800.00");

        let staples = rows.iter().find(|r| r.number == "SK-INV-30418").unwrap();
        assert_eq!(staples.total, 20_050_000);
        assert_eq!(staples.outstanding, None);
        assert_eq!(staples.outstanding_text(), "Not posted");

        let autox = rows.iter().find(|r| r.number == "AX-2026-0112").unwrap();
        assert_eq!(autox.status, BillStatus::Paid);
        assert_eq!(autox.outstanding, Some(0));
        // Paid bills are never overdue.
        assert_eq!(autox.overdue_days, 0);
        assert_eq!(autox.outstanding_text(), "\u{2013}");
    }

    #[test]
    fn a_late_posted_bill_is_overdue_and_sorts_first() {
        let state = books();
        // Ten days after Kenya Power's due date.
        let rows = bill_rows(&state, &d("2026-02-26"));
        let all = filtered(&rows, BillFilter::All);
        assert_eq!(all[0].number, "KPLC-0226-77140");
        assert_eq!(all[0].overdue_days, 10);
        assert_eq!(overdue_text(all[0].overdue_days), "Overdue 10 days");
        assert_eq!(overdue_text(1), "Overdue 1 day");
        // Then newest bill date first.
        let rest: Vec<_> = all[1..].iter().map(|r| r.number.as_str()).collect();
        assert_eq!(rest, ["SK-INV-30418", "AX-2026-0112"]);
    }

    #[test]
    fn the_filter_counts_and_keeps_its_status() {
        let state = books();
        let rows = bill_rows(&state, &as_of());
        let counts: Vec<_> = FILTERS.into_iter().map(|f| filter_count(&rows, f)).collect();
        assert_eq!(counts, [3, 0, 1, 1, 1, 0]);
        let to_pay = filtered(&rows, BillFilter::Status(BillStatus::Posted));
        assert_eq!(to_pay.len(), 1);
        assert_eq!(to_pay[0].number, "KPLC-0226-77140");
        assert_eq!(filtered(&rows, BillFilter::Status(BillStatus::Draft)), vec![]);
        assert_eq!(
            no_match_text(BillFilter::Status(BillStatus::Posted)),
            "No bills are posted, to pay."
        );
        assert_eq!(no_match_text(BillFilter::All), "No bills are here.");
    }

    #[test]
    fn vendors_list_by_name_with_what_is_owed() {
        let state = books();
        let rows = vendor_rows(&state);
        let names: Vec<_> = rows.iter().map(|r| r.name.as_str()).collect();
        let mut sorted = names.clone();
        sorted.sort_unstable();
        assert_eq!(names, sorted);
        assert_eq!(rows.len(), state.vendors.len());
        let power = rows.iter().find(|r| r.name == "Kenya Power").unwrap();
        assert_eq!((power.bills, power.outstanding), (1, 3_980_000));
        let autox = rows.iter().find(|r| r.name == "AutoXpress Ltd").unwrap();
        assert_eq!((autox.bills, autox.outstanding), (1, 0));
        // Staples is approved, not posted: nothing owed in the books yet.
        let staples = rows.iter().find(|r| r.name == "Staples Kenya").unwrap();
        assert_eq!((staples.bills, staples.outstanding), (1, 0));
        assert!(power.flow_node.is_some());
        assert_eq!(
            vendors_meta(&rows),
            format!("{} suppliers \u{b7} 39,800.00 owed", rows.len())
        );
    }

    #[test]
    fn the_vendor_form_needs_a_name() {
        let blank = NewVendorForm {
            name: "  ".into(),
            payment_details: String::new(),
        };
        assert_eq!(
            blank.command().unwrap_err(),
            "Enter the supplier\u{2019}s name as it appears on the bill."
        );
        let form = NewVendorForm {
            name: " Nairobi Printers ".into(),
            payment_details: " Paybill 400200 ".into(),
        };
        let LedgerCommand::VendorAdd { input } = form.command().unwrap() else {
            panic!("not vendor.add");
        };
        assert_eq!(input.name, "Nairobi Printers");
        assert_eq!(input.payment_details.as_deref(), Some("Paybill 400200"));
        assert_eq!(input.id, None);
        assert_eq!(input.node_id, None);
        let no_details = NewVendorForm {
            name: "X".into(),
            payment_details: " ".into(),
        };
        let LedgerCommand::VendorAdd { input } = no_details.command().unwrap() else {
            panic!("not vendor.add");
        };
        assert_eq!(input.payment_details, None);
    }

    #[test]
    fn dates_read_day_first() {
        assert_eq!(parse_day_month_year(" "), Ok(None));
        assert_eq!(parse_day_month_year("4/2/2026"), Ok(Some(d("2026-02-04"))));
        assert_eq!(parse_day_month_year("04.02.2026"), Ok(Some(d("2026-02-04"))));
        assert_eq!(
            parse_day_month_year("2026-02-04").unwrap_err(),
            "Enter the date as day/month/year, for example 18/01/2026."
        );
        assert_eq!(
            parse_day_month_year("4/2/26").unwrap_err(),
            "Write the year in full, for example 2026."
        );
        assert_eq!(
            parse_day_month_year("4/13/2026").unwrap_err(),
            "There is no month 13. Put the day first, then the month: dd/mm/yyyy."
        );
        assert_eq!(
            parse_day_month_year("30/02/2026").unwrap_err(),
            "February 2026 has only 28 days."
        );
        assert_eq!(
            parse_day_month_year("0/02/2026").unwrap_err(),
            "There is no day 0. Enter a day from 1."
        );
        assert_eq!(iso_to_day_month_year(&d("2026-01-18")), "18/01/2026");
    }

    #[test]
    fn a_new_form_starts_on_the_unrestricted_fund_due_in_30_days() {
        let state = books();
        let form = NewBillForm::new(&state, &as_of());
        assert_eq!(form.bill_date, "04/02/2026");
        assert_eq!(form.due_date, "06/03/2026");
        assert_eq!(form.currency, "KES");
        assert!(!form.foreign());
        assert_eq!(form.lines.len(), 1);
        assert_eq!(
            form.lines[0].fund_id.as_ref().map(FundId::as_str),
            Some(UNRESTRICTED_FUND_ID)
        );
        assert_eq!(form.total(), 0);
    }

    #[test]
    fn lines_add_remove_and_total_live() {
        let state = books();
        let mut form = NewBillForm::new(&state, &as_of());
        form.lines[0].account_id = Some(account_id("7100"));
        form.lines[0].amount = "1,250.50".into();
        form.add_line(&state);
        assert_eq!(form.lines.len(), 2);
        assert_eq!(form.lines[1].account_id, Some(account_id("7100")));
        assert_eq!(form.lines[1].amount, "");
        form.lines[1].amount = "100".into();
        assert_eq!(form.total(), 135_050);
        form.lines[1].amount = "-5".into();
        assert_eq!(form.total(), 125_050);
        form.lines[1].amount = "abc".into();
        assert_eq!(form.total(), 125_050);
        form.remove_line(0);
        assert_eq!(form.lines.len(), 1);
        form.remove_line(0);
        assert_eq!(form.lines.len(), 1, "the last line stays");
    }

    #[test]
    fn changing_the_fund_drops_or_picks_the_grant() {
        let state = books();
        let restricted = state
            .funds
            .iter()
            .find(|f| {
                state.grants.iter().filter(|g| g.fund_id == f.id).count() == 1
            })
            .unwrap();
        let only = state.grants.iter().find(|g| g.fund_id == restricted.id).unwrap();
        let mut line = LineDraft::new(&state);
        line.set_fund(&state, restricted.id.clone());
        assert_eq!(line.grant_id.as_ref(), Some(&only.id));
        // A fund with several grants or none drops the grant of another fund.
        let mut state = state.clone();
        state.funds.push(ledger_core::types::Fund {
            id: FundId::from("fund-no-grants"),
            name: "No grants".into(),
            restriction: ledger_core::types::FundRestriction::TemporarilyRestricted,
        });
        line.set_fund(&state, FundId::from("fund-no-grants"));
        assert_eq!(line.grant_id, None);
        // Choosing a grant then switching back to a one-grant fund picks that grant.
        line.set_fund(&state, restricted.id.clone());
        assert_eq!(line.grant_id.as_ref(), Some(&only.id));
        assert_eq!(
            grants_for(&state, Some(&restricted.id)),
            vec![(only.id.clone(), only.name.clone())]
        );
    }

    #[test]
    fn line_and_cash_pickers_offer_posting_accounts() {
        let state = books();
        let lines = line_accounts(&state);
        assert_ne!(lines, vec![]);
        for (id, label) in &lines {
            let a = state.account(id).unwrap();
            assert!(!a.is_group && a.is_active);
            assert!(matches!(
                a.account_type,
                ledger_core::types::AccountType::Expense | ledger_core::types::AccountType::Asset
            ));
            assert_eq!(label, &format!("{} \u{b7} {}", a.code, a.name));
        }
        // Assets come before expenses, as in TYPE_LABEL.
        let first = state.account(&lines[0].0).unwrap();
        assert_eq!(first.account_type, ledger_core::types::AccountType::Asset);
        let cash = cash_accounts(&state);
        assert!(cash.iter().any(|(id, _)| id == &account_id("1120")));
        assert!(cash.iter().all(|(id, _)| state.account(id).unwrap().is_cash));
    }

    #[test]
    fn the_form_checks_every_field_in_order() {
        let state = books();
        let mut form = NewBillForm::new(&state, &as_of());
        form.bill_date = "10/02/2026".into();
        form.due_date = "09/02/2026".into();
        let errors = form.command(&state).unwrap_err();
        assert_eq!(errors.title(), "Fix these before saving");
        assert_eq!(
            errors.summary(),
            [
                "Vendor: Choose the supplier, or add a new one.",
                "Bill number: Enter the invoice number printed on the bill.",
                "Due date: The due date cannot be before the bill date.",
                "Line 1 description: Say what this was for.",
                "Line 1 amount: Enter the amount.",
                "Line 1 account: Choose what kind of cost this is.",
            ]
        );
        assert_eq!(
            errors.line(0, LineField::Amount).as_deref(),
            Some("Enter the amount.")
        );
    }

    #[test]
    fn the_form_refuses_a_duplicate_number_and_a_missing_rate() {
        let state = books();
        let power = bill_by_number(&state, "KPLC-0226-77140");
        let mut form = NewBillForm::new(&state, &as_of());
        form.vendor = Some(power.vendor_id.clone());
        form.number = " kplc-0226-77140 ".into();
        form.currency = "USD".into();
        form.lines[0].description = "Power".into();
        form.lines[0].amount = "0".into();
        form.lines[0].account_id = Some(account_id("7300"));
        let errors = form.command(&state).unwrap_err();
        assert_eq!(
            errors.field(BillField::Number).as_deref(),
            Some("This supplier already has a bill with this number.")
        );
        assert_eq!(
            errors.field(BillField::FxRate).as_deref(),
            Some("Enter how many KES one USD is worth.")
        );
        assert_eq!(
            errors.line(0, LineField::Amount).as_deref(),
            Some("The amount must be more than zero.")
        );
        form.fx_rate = "-3".into();
        assert_eq!(
            form.command(&state).unwrap_err().field(BillField::FxRate).as_deref(),
            Some("Enter a positive number, like 129.45.")
        );
    }

    #[test]
    fn restricted_money_needs_its_grant() {
        let state = books();
        let restricted = state
            .funds
            .iter()
            .find(|f| f.id.as_str() != UNRESTRICTED_FUND_ID)
            .unwrap();
        let mut form = NewBillForm::new(&state, &as_of());
        form.lines[0].fund_id = Some(restricted.id.clone());
        form.lines[0].grant_id = None;
        let errors = form.command(&state).unwrap_err();
        assert_eq!(
            errors.line(0, LineField::Grant).as_deref(),
            Some("Restricted money needs the grant it comes from.")
        );
    }

    #[test]
    fn a_valid_form_builds_bill_create() {
        let state = books();
        let power = bill_by_number(&state, "KPLC-0226-77140");
        let mut form = NewBillForm::new(&state, &as_of());
        form.vendor = Some(power.vendor_id.clone());
        form.number = " KPLC-0326-1 ".into();
        form.currency = "USD".into();
        form.fx_rate = "1,129.45".into();
        form.lines[0].description = " Electricity ".into();
        form.lines[0].amount = "398".into();
        form.lines[0].account_id = Some(account_id("7300"));
        form.lines[0].dimension_id = Some(DimensionId::from("dim-loc-nairobi"));
        let LedgerCommand::BillCreate { input } = form.command(&state).unwrap() else {
            panic!("not bill.create");
        };
        assert_eq!(input.vendor_id, power.vendor_id);
        assert_eq!(input.number, "KPLC-0326-1");
        assert_eq!(input.bill_date, d("2026-02-04"));
        assert_eq!(input.due_date, d("2026-03-06"));
        assert_eq!(input.currency, "USD");
        assert!((input.fx_rate - 1129.45).abs() < f64::EPSILON);
        assert_eq!(input.proof_ids, Vec::<String>::new());
        assert_eq!(input.lines.len(), 1);
        let l = &input.lines[0];
        assert_eq!(l.description, "Electricity");
        assert!((l.amount - 39_800.0).abs() < f64::EPSILON);
        assert_eq!(l.grant_id, None);
        assert_eq!(l.dimension_ids, vec![DimensionId::from("dim-loc-nairobi")]);
    }

    #[test]
    fn status_maps_to_actions() {
        let state = books();
        let me = demo_actor();
        let staples = bill_by_number(&state, "SK-INV-30418");
        let (text, actions) = next_step(staples, Some(&me), None);
        assert_eq!(actions, [BillAction::Post]);
        assert!(text.starts_with("Approved by Demo User (you)."), "{text}");

        let power = bill_by_number(&state, "KPLC-0226-77140");
        let (text, actions) = next_step(power, Some(&me), Some(3_980_000));
        assert_eq!(actions, [BillAction::Pay]);
        assert_eq!(text, "Posted. KES 39,800.00 is still owed, due 16 Feb 2026.");

        let autox = bill_by_number(&state, "AX-2026-0112");
        let (text, actions) = next_step(autox, Some(&me), Some(0));
        assert_eq!(actions, vec![]);
        assert_eq!(text, "Paid in full. Nothing more to do.");

        // A draft: its creator cannot approve it, an approver can, others wait.
        let mut draft = staples.clone();
        draft.status = BillStatus::Draft;
        draft.approved_by = None;
        let (_, actions) = next_step(&draft, Some(&me), None);
        assert_eq!(actions, [BillAction::Approve]);
        draft.created_by = me.id.clone();
        let (text, actions) = next_step(&draft, Some(&me), None);
        assert_eq!(actions, vec![]);
        assert!(text.starts_with("You entered this bill"), "{text}");
        let clerk = Actor {
            id: ActorId::from("clerk"),
            name: "Clerk".into(),
            can_post: true,
            can_approve: false,
            can_close: false,
        };
        let (text, actions) = next_step(&draft, Some(&clerk), None);
        assert_eq!(actions, vec![]);
        assert_eq!(text, "Waiting for a manager with approval rights.");

        let mut void = staples.clone();
        void.status = BillStatus::Void;
        assert_eq!(
            next_step(&void, Some(&me), None),
            ("This bill was voided.".to_owned(), vec![])
        );
    }

    #[test]
    fn approve_and_post_build_their_commands() {
        let id = BillId::from("b-1");
        assert!(matches!(
            BillAction::Approve.command(&id),
            Some(LedgerCommand::BillApprove { bill_id }) if bill_id == id
        ));
        assert!(matches!(
            BillAction::Post.command(&id),
            Some(LedgerCommand::BillPost { bill_id, override_reason: None }) if bill_id == id
        ));
        assert!(BillAction::Pay.command(&id).is_none());
        assert_eq!(BillAction::Post.label(), "Post to the books");
        assert_eq!(BillAction::Pay.done(), "Payment recorded.");
    }

    #[test]
    fn the_paid_demo_bill_shows_its_entries_and_trail() {
        let state = books();
        let autox = bill_by_number(&state, "AX-2026-0112");
        let detail = bill_detail(&state, &autox.id, Some(&demo_actor()), &as_of()).unwrap();
        assert_eq!(detail.vendor, "AutoXpress Ltd");
        assert_eq!((detail.total, detail.outstanding, detail.paid), (4_800_000, Some(0), 4_800_000));
        assert_eq!(detail.lines.len(), 1);
        let tyres = state.account(&account_id("7200")).unwrap();
        assert_eq!(detail.lines[0].account, format!("7200 {}", tyres.name));
        let posting = detail.posting.as_ref().unwrap();
        assert!(posting.balanced());
        let payable = &state.settings.system_accounts.accounts_payable;
        assert_eq!(posting.movement(payable), -4_800_000);
        assert_eq!(detail.payments.len(), 1);
        let pay = &detail.payments[0];
        assert_eq!(pay.amount, 4_800_000);
        assert!(pay.source.ends_with("\u{b7} EFT 88213"), "{}", pay.source);
        let entry = pay.entry.as_ref().unwrap();
        assert!(entry.balanced());
        assert_eq!(entry.movement(payable), 4_800_000);
        assert_eq!(entry.movement(&account_id("1120")), -4_800_000);
        assert_eq!(
            pay.fund_split,
            vec![(
                FundId::from(UNRESTRICTED_FUND_ID),
                state.funds.iter().find(|f| f.id.as_str() == UNRESTRICTED_FUND_ID).unwrap().name.clone(),
                4_800_000
            )]
        );
        let steps: Vec<_> = detail.trail.iter().map(|t| t.what.as_str()).collect();
        assert_eq!(steps.len(), 4, "{steps:?}");
        assert_eq!(steps[0], "Entered bill AX-2026-0112");
        assert_eq!(steps[1], "Approved");
        assert!(steps[2].starts_with("Posted as JE-"), "{}", steps[2]);
        assert!(steps[3].starts_with("Paid 48,000.00 as JE-"), "{}", steps[3]);
        assert_eq!(detail.trail[0].who, "FlowTrace");
        assert_eq!(detail.trail[1].who, "Demo User (you)");
        assert_eq!(detail.actions, vec![]);
        assert!(bill_detail(&state, &BillId::from("nope"), None, &as_of()).is_none());
    }

    #[test]
    fn the_pay_form_defaults_to_what_is_owed_and_checks_it() {
        let id = BillId::from("b-1");
        let form = PayForm::new(Some(3_980_000), &as_of());
        assert_eq!(form.amount, "39800.00");
        assert_eq!(form.date, "04/02/2026");
        let errors = form.command(&id, 3_980_000).unwrap_err();
        assert_eq!(
            errors.cash.as_deref(),
            Some("Choose the account the money left from.")
        );
        let mut form = form;
        form.cash_account = Some(account_id("1120"));
        form.amount = "40,000".into();
        assert_eq!(
            form.command(&id, 3_980_000).unwrap_err().amount.as_deref(),
            Some("Only 39,800.00 is outstanding.")
        );
        form.amount = String::new();
        form.date = String::new();
        let errors = form.command(&id, 3_980_000).unwrap_err();
        assert_eq!(errors.amount.as_deref(), Some("Enter the amount paid."));
        assert_eq!(errors.date.as_deref(), Some("Enter the date the money left."));
        form.amount = "0".into();
        assert_eq!(
            form.command(&id, 3_980_000).unwrap_err().amount.as_deref(),
            Some("The amount must be more than zero.")
        );
        form.amount = "x".into();
        assert_eq!(
            form.command(&id, 3_980_000).unwrap_err().amount.as_deref(),
            Some("Enter a number, like 39800 or 39,800.00.")
        );
        form.amount = "10,000.50".into();
        form.date = "5/2/2026".into();
        form.reference = " EFT 1 ".into();
        let LedgerCommand::BillPay {
            bill_id,
            date,
            cash_account_id,
            amount,
            reference,
        } = form.command(&id, 3_980_000).unwrap()
        else {
            panic!("not bill.pay");
        };
        assert_eq!(bill_id, id);
        assert_eq!(date, "2026-02-05");
        assert_eq!(cash_account_id, account_id("1120"));
        assert!((amount - 1_000_050.0).abs() < f64::EPSILON);
        assert_eq!(reference.value().map(String::as_str), Some("EFT 1"));
        form.reference = "  ".into();
        let LedgerCommand::BillPay { reference, .. } = form.command(&id, 3_980_000).unwrap() else {
            panic!("not bill.pay");
        };
        assert_eq!(reference.value(), None);
    }
}
