//! View models of the Spend entry's accounts-payable half: vendors, the bills list with its
//! status filter, the new-bill form, the bill detail with its status-to-actions mapping, its
//! journal entries and payments, and the payment form. Plain structs, no GPUI; the panel in
//! `panels::bills` only draws them.
//!
//! Behaviour mirrors the web screens `flowtrace/src/routes/_authenticated/spend/bills.index.tsx`,
//! `bills.new.tsx`, `bills.$id.tsx` and `-shared.tsx`; the rules themselves stay in
//! `ledger-core` (`add_vendor`, `create_bill`, `approve_bill`, `post_bill`, `pay_bill`,
//! `outstanding`), whose messages are shown as they come.

use ledger_core::LedgerCommand;
use ledger_core::Minor;
use ledger_core::engine::{NewBill, NewCodedLine, NewVendor};
use ledger_core::money::parse_money;
use ledger_core::types::{
    AccountId, Actor, AppliesTo, Bill, BillId, BillStatus, DimensionId, EntryId, FundId, GrantId,
    IsoDate, LedgerState, VendorId,
};

/// `BILL_STATUS_LABEL` (`-shared.tsx:60-66`).
#[must_use]
pub fn status_label(_status: BillStatus) -> &'static str {
    ""
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
pub fn status_tone(_status: BillStatus) -> Tone {
    Tone::Quiet
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
        ""
    }

    /// Whether a bill in `status` shows under this filter.
    #[must_use]
    pub fn matches(self, _status: BillStatus) -> bool {
        false
    }
}

/// `daysBetween` (`-shared.tsx:130-134`): whole days from `a` to `b`, negative when `b` is
/// earlier.
#[must_use]
pub fn days_between(_a: &IsoDate, _b: &IsoDate) -> i64 {
    0
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
        String::new()
    }
}

/// `OverdueFlag` (`-shared.tsx:121-128`): `Overdue 1 day`, `Overdue 5 days`.
#[must_use]
pub fn overdue_text(_days: i64) -> String {
    String::new()
}

/// `rowsFor` (`bills.index.tsx:41-55`): every bill in the books' order, with its total, what is
/// still owed (posted and paid bills only) and how late it is at `as_of`.
#[must_use]
pub fn bill_rows(_state: &LedgerState, _as_of: &IsoDate) -> Vec<BillRow> {
    Vec::new()
}

/// The rows `filter` shows, most overdue first, then the newest bill date first
/// (`bills.index.tsx:66-73`).
#[must_use]
pub fn filtered(_rows: &[BillRow], _filter: BillFilter) -> Vec<BillRow> {
    Vec::new()
}

/// The count beside a filter (`bills.index.tsx:61-65`).
#[must_use]
pub fn filter_count(_rows: &[BillRow], _filter: BillFilter) -> usize {
    0
}

/// What the list says when a filter shows nothing (`bills.index.tsx:136-146`).
#[must_use]
pub fn no_match_text(_filter: BillFilter) -> String {
    String::new()
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
    /// The flow node's name when the vendor stands for one.
    pub flow_node: Option<String>,
    /// How many bills the vendor has.
    pub bills: usize,
    /// What is still owed on the vendor's posted bills, in base currency at each bill's rate.
    pub outstanding: Minor,
}

/// The vendors in name order (`bills.new.tsx`, `vendorOptions`), each with its bill count and
/// what is still owed on its posted bills.
#[must_use]
pub fn vendor_rows(_state: &LedgerState) -> Vec<VendorRow> {
    Vec::new()
}

/// The vendors section meta: `5 suppliers · 39,800.00 owed`.
#[must_use]
pub fn vendors_meta(_rows: &[VendorRow]) -> String {
    String::new()
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
    /// The `vendor.add` command, or the name field's error.
    ///
    /// # Errors
    ///
    /// The message shown under the name field when it is blank (`bills.new.tsx`, `check`).
    pub fn command(&self) -> Result<LedgerCommand, String> {
        Err(String::new())
    }
}

/// `parseDayMonthYear` (`flowtrace/src/components/ui/date-field.tsx:50-80`): `Ok(None)` for an
/// empty field, `Ok(Some(date))` for a valid `dd/mm/yyyy`, otherwise the plain-words problem.
///
/// # Errors
///
/// The message the web field shows for a malformed or impossible date.
pub fn parse_day_month_year(_text: &str) -> Result<Option<IsoDate>, String> {
    Ok(None)
}

/// `isoToDayMonthYear` (`date-field.tsx:36-39`): `2026-01-18` to `18/01/2026`.
#[must_use]
pub fn iso_to_day_month_year(_date: &IsoDate) -> String {
    String::new()
}

/// `addDays` (`bills.new.tsx:49-53`).
#[must_use]
pub fn add_days(date: &IsoDate, _n: i64) -> IsoDate {
    date.clone()
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
    pub fn new(_state: &LedgerState) -> Self {
        Self::default()
    }

    /// The line "Add another line" adds after this one: same account, fund, grant and
    /// dimension, empty description and amount (`-shared.tsx:551-568`).
    #[must_use]
    pub fn following(&self) -> Self {
        Self::default()
    }

    /// `withFund` (`-shared.tsx:373-378`): changing the fund drops a grant of another fund and
    /// picks the fund's only grant.
    pub fn set_fund(&mut self, _state: &LedgerState, fund: FundId) {
        self.fund_id = Some(fund);
    }
}

/// `AccountSelect` with `types={['expense', 'asset']}` (`pickers.tsx:25-44`,
/// `bills.new.tsx`): active posting accounts of those types, by type in `TYPE_LABEL` order, then
/// by code; labels `code · name`.
#[must_use]
pub fn line_accounts(_state: &LedgerState) -> Vec<(AccountId, String)> {
    Vec::new()
}

/// `AccountSelect cashOnly` (`pickers.tsx:25-44`): the active cash accounts, by code.
#[must_use]
pub fn cash_accounts(_state: &LedgerState) -> Vec<(AccountId, String)> {
    Vec::new()
}

/// `GrantSelect fundId` (`pickers.tsx:57-69`): the grants on `fund`.
#[must_use]
pub fn grants_for(_state: &LedgerState, _fund: Option<&FundId>) -> Vec<(GrantId, String)> {
    Vec::new()
}

/// `linesTotal` (`-shared.tsx:369-371`): the lines' amounts, a line that does not parse or is
/// negative counting as zero.
#[must_use]
pub fn lines_total(_lines: &[LineDraft]) -> Minor {
    0
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
        ""
    }

    /// `ErrorSummary`'s items (`-shared.tsx:615-630`): `Bill number: Enter ...`,
    /// `Line 1 amount: Enter ...`.
    #[must_use]
    pub fn summary(&self) -> Vec<String> {
        Vec::new()
    }
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
    pub fn new(state: &LedgerState, _as_of: &IsoDate) -> Self {
        Self {
            vendor: None,
            number: String::new(),
            bill_date: String::new(),
            due_date: String::new(),
            currency: String::new(),
            fx_rate: String::new(),
            lines: Vec::new(),
            base: state.settings.base_currency.clone(),
        }
    }

    /// Whether the bill is in another currency than the books', so it needs a rate.
    #[must_use]
    pub fn foreign(&self) -> bool {
        false
    }

    /// The live total under the lines.
    #[must_use]
    pub fn total(&self) -> Minor {
        0
    }

    /// "Add another line": a line following the last one.
    pub fn add_line(&mut self, _state: &LedgerState) {}

    /// Removes line `index`; the last line stays (`-shared.tsx:421-432`).
    pub fn remove_line(&mut self, _index: usize) {}

    /// The `bill.create` command, or what to fix first (`bills.new.tsx`, `check` and `save`).
    ///
    /// # Errors
    ///
    /// [`BillErrors`] with every field and line problem, in the form's order.
    pub fn command(&self, _state: &LedgerState) -> Result<LedgerCommand, BillErrors> {
        Err(BillErrors::default())
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
        ""
    }

    /// The notice after the command went through (`bills.$id.tsx`, each dialog's `setDone`).
    #[must_use]
    pub fn done(self) -> &'static str {
        ""
    }

    /// The command of Approve or Post; Pay goes through [`PayForm::command`].
    #[must_use]
    pub fn command(self, _bill: &BillId) -> Option<LedgerCommand> {
        None
    }
}

/// "What happens next" (`bills.$id.tsx`): the sentence for the bill's status and the actions
/// the actor may take. A draft is approved only by someone with approval rights who did not
/// enter it; approved bills post; posted bills take payments; paid and void bills take nothing.
#[must_use]
pub fn next_step(
    _bill: &Bill,
    _actor: Option<&Actor>,
    _outstanding: Option<Minor>,
) -> (String, Vec<BillAction>) {
    (String::new(), Vec::new())
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

/// One line of a journal entry under the bill.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntryLineRow {
    /// The account posted to, `code · name`.
    pub account: AccountId,
    /// The account's label.
    pub account_label: String,
    /// Fund name.
    pub fund: String,
    /// Debit in the line's currency.
    pub debit: Minor,
    /// Credit in the line's currency.
    pub credit: Minor,
    /// Signed debit-minus-credit in base currency.
    pub base_amount: Minor,
}

/// A journal entry the bill made: the accrual when posted, or a payment's entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntryView {
    /// Entry id.
    pub id: EntryId,
    /// Entry number, `JE-0042`.
    pub number: String,
    /// Accounting date.
    pub date: IsoDate,
    /// Memo.
    pub memo: String,
    /// Lines in posting order.
    pub lines: Vec<EntryLineRow>,
}

impl EntryView {
    /// Whether the base amounts sum to zero.
    #[must_use]
    pub fn balanced(&self) -> bool {
        false
    }

    /// The net base-currency movement on `account` in this entry (debit positive).
    #[must_use]
    pub fn movement(&self, _account: &AccountId) -> Minor {
        0
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

/// The detail of bill `id` as `actor` sees it at `as_of`, or `None` when the books do not hold
/// it.
#[must_use]
pub fn bill_detail(
    _state: &LedgerState,
    _id: &BillId,
    _actor: Option<&Actor>,
    _as_of: &IsoDate,
) -> Option<BillDetail> {
    None
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
    /// The form as `openPay` fills it: the amount owed with two decimals, dated `as_of`.
    #[must_use]
    pub fn new(_outstanding: Option<Minor>, _as_of: &IsoDate) -> Self {
        Self {
            amount: String::new(),
            date: String::new(),
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
    pub fn command(&self, _bill: &BillId, _outstanding: Minor) -> Result<LedgerCommand, PayErrors> {
        Err(PayErrors::default())
    }
}

/// The outstanding amount of `bill`, as the web reads it: only once posted or paid.
#[must_use]
pub fn outstanding_of(_state: &LedgerState, _bill: &Bill) -> Option<Minor> {
    None
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
        assert!(filtered(&rows, BillFilter::Status(BillStatus::Draft)).is_empty());
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
        line.set_fund(&state, FundId::from(UNRESTRICTED_FUND_ID));
        assert_eq!(line.grant_id, None);
        assert_eq!(
            grants_for(&state, Some(&restricted.id)),
            vec![(only.id.clone(), only.name.clone())]
        );
    }

    #[test]
    fn line_and_cash_pickers_offer_posting_accounts() {
        let state = books();
        let lines = line_accounts(&state);
        assert!(!lines.is_empty());
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
        assert!(input.proof_ids.is_empty());
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
        assert!(actions.is_empty());
        assert_eq!(text, "Paid in full. Nothing more to do.");

        // A draft: its creator cannot approve it, an approver can, others wait.
        let mut draft = staples.clone();
        draft.status = BillStatus::Draft;
        draft.approved_by = None;
        let (_, actions) = next_step(&draft, Some(&me), None);
        assert_eq!(actions, [BillAction::Approve]);
        draft.created_by = me.id.clone();
        let (text, actions) = next_step(&draft, Some(&me), None);
        assert!(actions.is_empty());
        assert!(text.starts_with("You entered this bill"), "{text}");
        let clerk = Actor {
            id: ActorId::from("clerk"),
            name: "Clerk".into(),
            can_post: true,
            can_approve: false,
            can_close: false,
        };
        let (text, actions) = next_step(&draft, Some(&clerk), None);
        assert!(actions.is_empty());
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
        assert!(detail.actions.is_empty());
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
