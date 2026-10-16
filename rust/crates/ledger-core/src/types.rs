//! Ledger domain types, mirroring `flowtrace/src/lib/ledger/types.ts`.
//!
//! Plain data with no I/O, shared by the desktop app, the SpacetimeDB module and its client
//! (`types.ts:1-3`). Money is integer minor units ([`Minor`]); the only floats are exchange
//! rates (`fx_rate`), which are written as JS numbers through [`crate::js_number`].
//!
//! The JSON shape is the TS one, byte for byte: record fields keep the TS `snake_case` names, the
//! three `camelCase` collections of [`LedgerState`] are renamed, enums are the TS string literals,
//! and a TS `T | null` field is an `Option` written as `null`. State structs deny unknown fields,
//! so a field missed here fails to load instead of being dropped. Input types
//! ([`LineInput`], [`EntryInput`]) leave optional fields out when they are `None`, as TS leaves
//! them `undefined`.

use std::collections::BTreeMap;
use std::fmt;

use serde::{Deserialize, Serialize};

pub use crate::env::Timestamp;
pub use crate::ids::{
    AccountId, ActorId, AuditEventId, BankAccountId, BillId, BudgetId, BudgetLineId, ClaimId,
    CodedLineId, DimensionId, EntryId, FlowNodeId, FundId, GrantId, ImportBatchId, LineId,
    OrganizationId, PaymentId, PeriodId, StatementLineId, VendorId,
};
pub use crate::money::Minor;

/// A calendar date as TS holds it, `YYYY-MM-DD` (`types.ts:9`).
///
/// A validated string, compared as a string as TS compares dates (`rust/fixtures/README.md`,
/// "Dates"); date arithmetic in UTC belongs to the engine.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct IsoDate(String);

/// A string that is not `YYYY-MM-DD`.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("not an ISO date (YYYY-MM-DD): {0:?}")]
pub struct IsoDateError(String);

impl IsoDate {
    /// Accepts `YYYY-MM-DD` with ASCII digits (shape only; calendar ranges are the engine's).
    ///
    /// # Errors
    ///
    /// [`IsoDateError`] for any other shape.
    pub fn parse(s: &str) -> Result<Self, IsoDateError> {
        let ok = s.len() == 10
            && s.bytes().enumerate().all(|(i, b)| {
                if i == 4 || i == 7 {
                    b == b'-'
                } else {
                    b.is_ascii_digit()
                }
            });
        if ok {
            Ok(Self(s.to_owned()))
        } else {
            Err(IsoDateError(s.to_owned()))
        }
    }

    /// `YYYY-MM-DD` from numeric parts, zero-padded; the caller keeps them in range.
    pub(crate) fn from_parts(year: i64, month: i64, day: i64) -> Self {
        Self(format!("{year:04}-{month:02}-{day:02}"))
    }

    /// The date as `YYYY-MM-DD`.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for IsoDate {
    type Error = IsoDateError;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        Self::parse(&s)
    }
}

impl From<IsoDate> for String {
    fn from(date: IsoDate) -> Self {
        date.0
    }
}

impl fmt::Display for IsoDate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// `AccountType` (`types.ts:11`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AccountType {
    /// `asset`
    Asset,
    /// `liability`
    Liability,
    /// `equity`
    Equity,
    /// `income`
    Income,
    /// `expense`
    Expense,
}

/// `NormalBalance` (`types.ts:12`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NormalBalance {
    /// `debit`
    Debit,
    /// `credit`
    Credit,
}

/// Functional classification, for the Statement of Functional Expenses (`types.ts:14-15`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExpenseFunction {
    /// `program`
    Program,
    /// `management`
    Management,
    /// `fundraising`
    Fundraising,
}

/// Cash-flow classification, for the cash flow statement (`types.ts:17-18`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CashFlowClass {
    /// `operating`
    Operating,
    /// `investing`
    Investing,
    /// `financing`
    Financing,
}

/// A chart-of-accounts account (`types.ts:20-34`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Account {
    /// Id, for example `acc-1110`.
    pub id: AccountId,
    /// Account code, for example `1110`.
    pub code: String,
    /// Display name.
    pub name: String,
    /// `type` in TS.
    #[serde(rename = "type")]
    pub account_type: AccountType,
    /// Side that increases the balance.
    pub normal_balance: NormalBalance,
    /// Header account above this one.
    pub parent_id: Option<AccountId>,
    /// Header accounts group others and cannot be posted to.
    pub is_group: bool,
    /// Inactive accounts take no new lines.
    pub is_active: bool,
    /// Bank, mobile money or petty cash: counts as cash.
    pub is_cash: bool,
    /// Functional classification of an expense account.
    pub expense_function: Option<ExpenseFunction>,
    /// Cash-flow classification.
    pub cash_flow_class: Option<CashFlowClass>,
}

/// `FundRestriction` (`types.ts:36-39`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FundRestriction {
    /// `unrestricted`
    Unrestricted,
    /// `temporarily_restricted`
    TemporarilyRestricted,
    /// `permanently_restricted`
    PermanentlyRestricted,
}

/// A fund (`types.ts:41-45`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Fund {
    /// Id, for example `fund-unrestricted`.
    pub id: FundId,
    /// Display name.
    pub name: String,
    /// Donor restriction.
    pub restriction: FundRestriction,
}

/// A grant (`types.ts:47-59`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Grant {
    /// Id.
    pub id: GrantId,
    /// Display name.
    pub name: String,
    /// The donor flow node, so the flow map and the ledger meet.
    pub donor_node_id: Option<FlowNodeId>,
    /// Donor display name.
    pub donor_name: String,
    /// Fund the grant belongs to.
    pub fund_id: FundId,
    /// Award in minor units of `currency`.
    pub award: Minor,
    /// ISO currency code.
    pub currency: String,
    /// First day spend may be charged.
    pub start_date: IsoDate,
    /// Last day spend may be charged.
    pub end_date: IsoDate,
    /// Free-text terms.
    pub terms: String,
}

/// `DimensionKind` (`types.ts:61`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DimensionKind {
    /// `project`
    Project,
    /// `department`
    Department,
    /// `location`
    Location,
    /// `donor`
    Donor,
}

/// A dimension value (`types.ts:63-69`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DimensionValue {
    /// Id.
    pub id: DimensionId,
    /// Which dimension.
    pub kind: DimensionKind,
    /// Display name.
    pub name: String,
    /// The flow node this value stands for, if any.
    pub node_id: Option<FlowNodeId>,
}

/// `PeriodStatus` (`types.ts:71`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PeriodStatus {
    /// `open`
    Open,
    /// `closed`
    Closed,
    /// `locked`
    Locked,
}

/// A fiscal period (`types.ts:73-80`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FiscalPeriod {
    /// Id, for example `period-2026-01`.
    pub id: PeriodId,
    /// Display name, for example `Jan 2026`.
    pub name: String,
    /// Fiscal year the period belongs to.
    pub fiscal_year: i32,
    /// First day.
    pub start_date: IsoDate,
    /// Last day.
    pub end_date: IsoDate,
    /// Open, closed or locked.
    pub status: PeriodStatus,
}

/// `EntryStatus` (`types.ts:82`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EntryStatus {
    /// `draft`
    Draft,
    /// `posted`
    Posted,
    /// `reversed`
    Reversed,
}

/// `SourceType` (`types.ts:84-95`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceType {
    /// `manual`
    Manual,
    /// `transaction`
    Transaction,
    /// `bill`
    Bill,
    /// `payment`
    Payment,
    /// `expense_claim`
    ExpenseClaim,
    /// `reimbursement`
    Reimbursement,
    /// `statement`
    Statement,
    /// `opening`
    Opening,
    /// `year_end_close`
    YearEndClose,
    /// `reversal`
    Reversal,
    /// `import`
    Import,
}

/// A journal line (`types.ts:97-114`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct JournalLine {
    /// Id.
    pub id: LineId,
    /// Entry the line belongs to.
    pub entry_id: EntryId,
    /// Account posted to.
    pub account_id: AccountId,
    /// One side is > 0 and the other is 0, in transaction currency.
    pub debit: Minor,
    /// See `debit`.
    pub credit: Minor,
    /// Transaction currency.
    pub currency: String,
    /// Units of base currency per unit of `currency`. 1 for base currency.
    #[serde(with = "crate::js_number")]
    pub fx_rate: f64,
    /// Signed debit-minus-credit in base currency, frozen when posted.
    pub base_amount: Minor,
    /// Fund charged.
    pub fund_id: FundId,
    /// Grant charged, if any.
    pub grant_id: Option<GrantId>,
    /// Dimension values tagged.
    pub dimension_ids: Vec<DimensionId>,
    /// Flow node the money moved through, if any.
    pub flow_node_id: Option<FlowNodeId>,
    /// Line memo.
    pub memo: Option<String>,
}

/// A journal entry header (`types.ts:116-133`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct JournalEntry {
    /// Id.
    pub id: EntryId,
    /// Entry number, for example `JE-0001`.
    pub number: String,
    /// Accounting date.
    pub date: IsoDate,
    /// Period the date falls in.
    pub period_id: PeriodId,
    /// Entry memo.
    pub memo: String,
    /// What produced the entry.
    pub source_type: SourceType,
    /// The source record's id; its kind follows `source_type`.
    pub source_id: Option<String>,
    /// Draft, posted or reversed.
    pub status: EntryStatus,
    /// Entry this one reverses.
    pub reversal_of: Option<EntryId>,
    /// Entry that reversed this one.
    pub reversed_by: Option<EntryId>,
    /// Set when a restricted-fund rule was overridden.
    pub override_reason: Option<String>,
    /// Who created it.
    pub created_by: ActorId,
    /// When it was created.
    pub created_at: Timestamp,
    /// Who posted it.
    pub posted_by: Option<ActorId>,
    /// When it was posted.
    pub posted_at: Option<Timestamp>,
}

/// A budget line (`types.ts:135-146`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BudgetLine {
    /// Id.
    pub id: BudgetLineId,
    /// Budget the line belongs to.
    pub budget_id: BudgetId,
    /// Null means every expense account.
    pub account_id: Option<AccountId>,
    /// Fund filter.
    pub fund_id: Option<FundId>,
    /// Grant filter.
    pub grant_id: Option<GrantId>,
    /// Dimension filter.
    pub dimension_id: Option<DimensionId>,
    /// Null means the whole fiscal year.
    pub period_id: Option<PeriodId>,
    /// Budgeted amount in base currency.
    pub amount: Minor,
}

/// `Budget['status']` (`types.ts:152`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BudgetStatus {
    /// `draft`
    Draft,
    /// `approved`
    Approved,
}

/// A budget (`types.ts:148-153`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Budget {
    /// Id.
    pub id: BudgetId,
    /// Display name.
    pub name: String,
    /// Fiscal year budgeted.
    pub fiscal_year: i32,
    /// Draft or approved.
    pub status: BudgetStatus,
}

/// A vendor (`types.ts:155-160`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Vendor {
    /// Id.
    pub id: VendorId,
    /// Display name.
    pub name: String,
    /// The flow node this vendor stands for, if any.
    pub node_id: Option<FlowNodeId>,
    /// Free-text payment details.
    pub payment_details: Option<String>,
}

/// `BillStatus` (`types.ts:162`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BillStatus {
    /// `draft`
    Draft,
    /// `approved`
    Approved,
    /// `posted`
    Posted,
    /// `paid`
    Paid,
    /// `void`
    Void,
}

/// A coded line of a bill or claim (`types.ts:164-172`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CodedLine {
    /// Id.
    pub id: CodedLineId,
    /// What was bought.
    pub description: String,
    /// Expense account.
    pub account_id: AccountId,
    /// Amount in the document's currency.
    pub amount: Minor,
    /// Fund charged.
    pub fund_id: FundId,
    /// Grant charged, if any.
    pub grant_id: Option<GrantId>,
    /// Dimension values tagged.
    pub dimension_ids: Vec<DimensionId>,
}

/// A vendor bill (`types.ts:174-189`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Bill {
    /// Id.
    pub id: BillId,
    /// Vendor billed by.
    pub vendor_id: VendorId,
    /// The vendor's bill number.
    pub number: String,
    /// Bill date.
    pub bill_date: IsoDate,
    /// Due date.
    pub due_date: IsoDate,
    /// Bill currency.
    pub currency: String,
    /// Units of base currency per unit of `currency`.
    #[serde(with = "crate::js_number")]
    pub fx_rate: f64,
    /// Lifecycle status.
    pub status: BillStatus,
    /// Coded lines.
    pub lines: Vec<CodedLine>,
    /// Posting entry, once posted.
    pub entry_id: Option<EntryId>,
    /// Proof references, matching verifications on the source document.
    pub proof_ids: Vec<String>,
    /// Who entered it.
    pub created_by: ActorId,
    /// Who approved it.
    pub approved_by: Option<ActorId>,
}

/// What a payment settles (`types.ts:200`), written `{ "kind": "bill", "id": "..." }`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "id",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum AppliesTo {
    /// `kind: 'bill'`
    Bill(BillId),
    /// `kind: 'expense_claim'`
    ExpenseClaim(ClaimId),
}

/// A payment out of a cash account (`types.ts:191-203`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Payment {
    /// Id.
    pub id: PaymentId,
    /// Payment date.
    pub date: IsoDate,
    /// The cash account money left from.
    pub cash_account_id: AccountId,
    /// Amount in `currency`.
    pub amount: Minor,
    /// Payment currency.
    pub currency: String,
    /// Units of base currency per unit of `currency`.
    #[serde(with = "crate::js_number")]
    pub fx_rate: f64,
    /// Bank or mobile-money reference.
    pub reference: Option<String>,
    /// The bill or claim settled.
    pub applies_to: AppliesTo,
    /// Posting entry.
    pub entry_id: EntryId,
    /// Who recorded it.
    pub created_by: ActorId,
}

/// `ClaimStatus` (`types.ts:205-211`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClaimStatus {
    /// `draft`
    Draft,
    /// `submitted`
    Submitted,
    /// `approved`
    Approved,
    /// `rejected`
    Rejected,
    /// `posted`
    Posted,
    /// `reimbursed`
    Reimbursed,
}

/// `ApprovalStep['action']` (`types.ts:216`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalAction {
    /// `submitted`
    Submitted,
    /// `approved`
    Approved,
    /// `rejected`
    Rejected,
    /// `posted`
    Posted,
    /// `reimbursed`
    Reimbursed,
}

/// One step in a claim's approval trail (`types.ts:213-218`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ApprovalStep {
    /// When.
    pub at: Timestamp,
    /// Who.
    pub by: ActorId,
    /// What they did.
    pub action: ApprovalAction,
    /// Their note.
    pub note: Option<String>,
}

/// An expense claim (`types.ts:220-236`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExpenseClaim {
    /// Id.
    pub id: ClaimId,
    /// Claim number, for example `EC-0001`.
    pub number: String,
    /// Who claims.
    pub claimant: ActorId,
    /// Claimant display name.
    pub claimant_name: String,
    /// Claim date.
    pub claim_date: IsoDate,
    /// Claim currency.
    pub currency: String,
    /// Units of base currency per unit of `currency`.
    #[serde(with = "crate::js_number")]
    pub fx_rate: f64,
    /// What the spend was for.
    pub purpose: String,
    /// Coded lines.
    pub lines: Vec<CodedLine>,
    /// Lifecycle status.
    pub status: ClaimStatus,
    /// Approval trail, oldest first.
    pub approvals: Vec<ApprovalStep>,
    /// Receipt file names.
    pub receipt_names: Vec<String>,
    /// Posting entry, once posted.
    pub entry_id: Option<EntryId>,
    /// Set when captured offline and synced later.
    pub captured_offline: bool,
}

/// `BankAccount['kind']` (`types.ts:241`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BankAccountKind {
    /// `bank`
    Bank,
    /// `mobile_money`
    MobileMoney,
    /// `cash`
    Cash,
}

/// A bank, mobile money or cash account (`types.ts:238-246`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BankAccount {
    /// Id.
    pub id: BankAccountId,
    /// Display name.
    pub name: String,
    /// Bank, mobile money or cash.
    pub kind: BankAccountKind,
    /// Provider, for example `Equity Bank` or `Safaricom M-Pesa`.
    pub provider: String,
    /// The GL cash account.
    pub account_id: AccountId,
    /// Account currency.
    pub currency: String,
    /// Masked number, for example `•••• 4821`.
    pub number_masked: String,
}

/// `StatementLine['match_kind']` (`types.ts:259`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MatchKind {
    /// `auto`
    Auto,
    /// `manual`
    Manual,
}

/// A bank or mobile-money statement line (`types.ts:248-260`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StatementLine {
    /// Id.
    pub id: StatementLineId,
    /// Account the statement belongs to.
    pub bank_account_id: BankAccountId,
    /// Value date.
    pub date: IsoDate,
    /// Statement text.
    pub description: String,
    /// Statement reference.
    pub reference: Option<String>,
    /// Signed: money in > 0, money out < 0, in the account's currency.
    pub amount: Minor,
    /// The import that brought the line in.
    pub import_batch: ImportBatchId,
    /// The journal line this statement line is matched to.
    pub matched_line_id: Option<LineId>,
    /// How it was matched.
    pub match_kind: Option<MatchKind>,
}

/// One audit event (`types.ts:262-269`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuditEvent {
    /// Id.
    pub id: AuditEventId,
    /// When.
    pub at: Timestamp,
    /// Who.
    pub by: ActorId,
    /// What, for example `fund.create`.
    pub action: String,
    /// The id of the record acted on; its kind follows `action`.
    pub subject: String,
    /// Free-text detail (for `account.update`, the JSON of the patch).
    pub detail: String,
}

/// `OrgLedgerSettings['basis']` (`types.ts:276`): accrual books, with a cash-basis report view.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Basis {
    /// `accrual`
    Accrual,
}

/// `OrgLedgerSettings['reporting_standard']` (`types.ts:278`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReportingStandard {
    /// `ifrs_sme`, the recommended default.
    IfrsSme,
    /// `ipsas`
    Ipsas,
}

/// System accounts the engine posts to (`types.ts:280-287`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SystemAccounts {
    /// Accounts payable.
    pub accounts_payable: AccountId,
    /// Expense claims payable.
    pub claims_payable: AccountId,
    /// Suspense.
    pub suspense: AccountId,
    /// Unrestricted net assets.
    pub net_assets_unrestricted: AccountId,
    /// Restricted net assets.
    pub net_assets_restricted: AccountId,
    /// Realised exchange gains and losses.
    pub fx_gain_loss: AccountId,
}

/// One organisation's ledger settings (`types.ts:271-288`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OrgLedgerSettings {
    /// Organisation.
    pub organization_id: OrganizationId,
    /// Base currency, for example `KES`.
    pub base_currency: String,
    /// 1 = January.
    pub fiscal_year_start_month: u8,
    /// Accounting basis.
    pub basis: Basis,
    /// Presentation standard.
    pub reporting_standard: ReportingStandard,
    /// System accounts the engine posts to.
    pub system_accounts: SystemAccounts,
}

/// One organisation's books (`types.ts:290-309`), in the order the TS engine keeps them.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LedgerState {
    /// Settings.
    pub settings: OrgLedgerSettings,
    /// Chart of accounts.
    pub accounts: Vec<Account>,
    /// Funds.
    pub funds: Vec<Fund>,
    /// Grants.
    pub grants: Vec<Grant>,
    /// Dimension values.
    pub dimensions: Vec<DimensionValue>,
    /// Fiscal periods.
    pub periods: Vec<FiscalPeriod>,
    /// Journal entry headers.
    pub entries: Vec<JournalEntry>,
    /// Journal lines of every entry.
    pub lines: Vec<JournalLine>,
    /// Budgets.
    pub budgets: Vec<Budget>,
    /// Budget lines (`budgetLines` in TS).
    #[serde(rename = "budgetLines")]
    pub budget_lines: Vec<BudgetLine>,
    /// Vendors.
    pub vendors: Vec<Vendor>,
    /// Bills.
    pub bills: Vec<Bill>,
    /// Payments.
    pub payments: Vec<Payment>,
    /// Expense claims.
    pub claims: Vec<ExpenseClaim>,
    /// Bank, mobile money and cash accounts (`bankAccounts` in TS).
    #[serde(rename = "bankAccounts")]
    pub bank_accounts: Vec<BankAccount>,
    /// Imported statement lines (`statementLines` in TS).
    #[serde(rename = "statementLines")]
    pub statement_lines: Vec<StatementLine>,
    /// Audit trail, oldest first.
    pub audit: Vec<AuditEvent>,
    /// Document number counters by prefix, for example `JE` and `EC`.
    pub counters: BTreeMap<String, u64>,
}

/// Who is acting, and what they may do (`types.ts:311-318`). Checked by every command.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Actor {
    /// Id.
    pub id: ActorId,
    /// Display name, used in permission messages.
    pub name: String,
    /// May post to the ledger.
    pub can_post: bool,
    /// May approve spend.
    pub can_approve: bool,
    /// May close periods.
    pub can_close: bool,
}

/// A TS optional nullable field (`?: T | null`): left out, `null`, or a value.
///
/// The engine reads such a field with `?? null`, so [`Nullish::value`] treats the first two
/// alike; the distinction only matters for writing the input back out. Pair with
/// `#[serde(default, skip_serializing_if = "Nullish::is_undefined")]`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum Nullish<T> {
    /// The field is left out (`undefined`).
    #[default]
    Undefined,
    /// The field is `null`.
    Null,
    /// The field holds a value.
    Value(T),
}

impl<T> Nullish<T> {
    /// The value, reading `undefined` and `null` alike (`?? null`).
    #[must_use]
    pub fn value(&self) -> Option<&T> {
        match self {
            Self::Value(v) => Some(v),
            Self::Undefined | Self::Null => None,
        }
    }

    /// True when the field is left out.
    #[must_use]
    pub fn is_undefined(&self) -> bool {
        matches!(self, Self::Undefined)
    }
}

impl<T> From<Option<T>> for Nullish<T> {
    fn from(value: Option<T>) -> Self {
        value.map_or(Self::Null, Self::Value)
    }
}

impl<T: Serialize> Serialize for Nullish<T> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::Value(v) => v.serialize(serializer),
            Self::Undefined | Self::Null => serializer.serialize_none(),
        }
    }
}

impl<'de, T: Deserialize<'de>> Deserialize<'de> for Nullish<T> {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Ok(Option::<T>::deserialize(deserializer)?.into())
    }
}

/// One line of an entry to post, as the caller sends it (`types.ts:320-331`).
///
/// Unvalidated input: `debit`, `credit` and `fx_rate` are the raw JS numbers (`10.5` is a valid
/// input that the engine rejects with `NOT_MINOR_UNITS`), so they are `f64` here and become
/// [`Minor`] through [`crate::money::assert_minor`]. A TS `?: T` field is an `Option` left out of
/// the JSON when `None`; a TS `?: T | null` field is a [`Nullish`], which keeps absent and `null`
/// apart so an input re-serialises exactly as sent.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LineInput {
    /// Account to post to.
    pub account_id: AccountId,
    /// Debit in transaction currency.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "crate::js_number::option"
    )]
    pub debit: Option<f64>,
    /// Credit in transaction currency.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "crate::js_number::option"
    )]
    pub credit: Option<f64>,
    /// Transaction currency; base currency when absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub currency: Option<String>,
    /// Units of base currency per unit of `currency`.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "crate::js_number::option"
    )]
    pub fx_rate: Option<f64>,
    /// Fund to charge.
    pub fund_id: FundId,
    /// Grant to charge.
    #[serde(default, skip_serializing_if = "Nullish::is_undefined")]
    pub grant_id: Nullish<GrantId>,
    /// Dimension values to tag.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dimension_ids: Option<Vec<DimensionId>>,
    /// Flow node the money moved through.
    #[serde(default, skip_serializing_if = "Nullish::is_undefined")]
    pub flow_node_id: Nullish<FlowNodeId>,
    /// Line memo.
    #[serde(default, skip_serializing_if = "Nullish::is_undefined")]
    pub memo: Nullish<String>,
}

/// An entry to post or save as a draft, as the caller sends it (`types.ts:333-341`).
///
/// `date` stays a plain string: the engine validates it and answers a bad one with a ledger
/// issue rather than a parse error.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EntryInput {
    /// Accounting date, `YYYY-MM-DD`.
    pub date: String,
    /// Entry memo.
    pub memo: String,
    /// Lines.
    pub lines: Vec<LineInput>,
    /// What produced the entry; `manual` when absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_type: Option<SourceType>,
    /// The source record's id.
    #[serde(default, skip_serializing_if = "Nullish::is_undefined")]
    pub source_id: Nullish<String>,
    /// Allows a restricted-fund rule to be overridden, with a reason.
    #[serde(default, skip_serializing_if = "Nullish::is_undefined")]
    pub override_reason: Nullish<String>,
}
