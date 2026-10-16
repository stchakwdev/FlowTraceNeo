//! The module's tables, column for column the tables of `spacetimedb/src/index.ts:42-165`.
//!
//! Every table is private (the default without `public`:
//! <https://docs.rs/spacetimedb/2.12.0/spacetimedb/attr.table.html>); clients read through the `my_*` views in
//! [`crate::views`]. Column names are the snake case of the TS columns, which is also what the
//! TS client queries (`flowtrace/src/lib/ledger/spacetime-backend.ts:128`, `org_id`).
//!
//! The row shape is the one the TS codec defines (`flowtrace/src/lib/ledger/spacetime-codec.ts:1-10`):
//! journal entries and lines get typed columns, every other collection is a doc row holding the
//! ledger-core type as JSON, and every ledger row is keyed `{org_id}:{id}` because ids are only
//! unique per organisation. `seq` keeps the state's array order. Why the rows are not
//! ledger-core types derived as SpacetimeDB types: `docs/adr/0003-module-tables.md`.

use ledger_core::types::{
    Account, AuditEvent, BankAccount, Bill, Budget, BudgetLine, DimensionValue, ExpenseClaim,
    FiscalPeriod, Fund, Grant, Payment, StatementLine, Vendor,
};
use serde::Serialize;
use serde::de::DeserializeOwned;
use spacetimedb::{Identity, Timestamp};

/// Identities allowed to open books for a new organisation (`index.ts:154-165`). The identity
/// that publishes the module becomes the first operator in `init`.
#[spacetimedb::table(accessor = operator)]
#[derive(Debug, Clone, PartialEq)]
pub struct Operator {
    /// The operator.
    #[primary_key]
    pub identity: Identity,
    /// When they were added.
    pub added_at: Timestamp,
}

/// One organisation's books (`index.ts:42-52`): settings, counters and the next row `seq`.
#[spacetimedb::table(accessor = ledger_org)]
#[derive(Debug, Clone, PartialEq)]
pub struct LedgerOrg {
    /// Organisation id, `^[A-Za-z0-9_-]{1,128}$` (`index.ts:533`).
    #[primary_key]
    pub org_id: String,
    /// `OrgLedgerSettings` as JSON.
    pub settings_json: String,
    /// The document number counters (`LedgerState.counters`) as JSON.
    pub counters_json: String,
    /// The `seq` the next inserted ledger row gets.
    pub next_seq: u64,
    /// Who opened the books.
    pub created_by: Identity,
    /// When the books were opened.
    pub created_at: Timestamp,
}

/// A member of an organisation and their permissions (`index.ts:54-66`). The reducers take the
/// actor from here, never from the caller (`index.ts:8-9`).
#[spacetimedb::table(accessor = member)]
#[derive(Debug, Clone, PartialEq)]
pub struct Member {
    /// `{org_id}:{identity hex}` (`index.ts:57`, `index.ts:359`).
    #[primary_key]
    pub key: String,
    /// The member's identity.
    #[index(btree)]
    pub identity: Identity,
    /// The organisation.
    #[index(btree)]
    pub org_id: String,
    /// The person behind approvals and the audit trail; fixed once given (`index.ts:614-617`).
    pub user_id: String,
    /// Display name.
    pub name: String,
    /// May post to the ledger.
    pub can_post: bool,
    /// May approve spend.
    pub can_approve: bool,
    /// May close periods, and manage members.
    pub can_close: bool,
}

/// A journal entry header with typed columns (`index.ts:68-90`, `spacetime-codec.ts:57-76`).
#[spacetimedb::table(accessor = journal_entry)]
#[derive(Debug, Clone, PartialEq)]
pub struct JournalEntryRow {
    /// `{org_id}:{id}`.
    #[primary_key]
    pub key: String,
    /// The organisation.
    #[index(btree)]
    pub org_id: String,
    /// `JournalEntry.id`.
    pub id: String,
    /// Position in `LedgerState.entries`.
    pub seq: u64,
    /// `JournalEntry.number`.
    pub number: String,
    /// `JournalEntry.date`, `YYYY-MM-DD`.
    pub date: String,
    /// `JournalEntry.period_id`.
    pub period_id: String,
    /// `JournalEntry.memo`.
    pub memo: String,
    /// `JournalEntry.source_type`, the TS string literal.
    pub source_type: String,
    /// `JournalEntry.source_id`.
    pub source_id: Option<String>,
    /// `JournalEntry.status`, the TS string literal.
    pub status: String,
    /// `JournalEntry.reversal_of`.
    pub reversal_of: Option<String>,
    /// `JournalEntry.reversed_by`.
    pub reversed_by: Option<String>,
    /// `JournalEntry.override_reason`.
    pub override_reason: Option<String>,
    /// `JournalEntry.created_by`.
    pub created_by: String,
    /// `JournalEntry.created_at`.
    pub created_at: String,
    /// `JournalEntry.posted_by`.
    pub posted_by: Option<String>,
    /// `JournalEntry.posted_at`.
    pub posted_at: Option<String>,
}

/// A journal line with typed columns, amounts as i64 minor units (`index.ts:92-112`,
/// `spacetime-codec.ts:78-95`).
#[spacetimedb::table(accessor = journal_line)]
#[derive(Debug, Clone, PartialEq)]
pub struct JournalLineRow {
    /// `{org_id}:{id}`.
    #[primary_key]
    pub key: String,
    /// The organisation.
    #[index(btree)]
    pub org_id: String,
    /// `JournalLine.id`.
    pub id: String,
    /// Position in `LedgerState.lines`.
    pub seq: u64,
    /// `JournalLine.entry_id`.
    pub entry_id: String,
    /// `JournalLine.account_id`.
    pub account_id: String,
    /// `JournalLine.debit`.
    pub debit: i64,
    /// `JournalLine.credit`.
    pub credit: i64,
    /// `JournalLine.currency`.
    pub currency: String,
    /// `JournalLine.fx_rate`.
    pub fx_rate: f64,
    /// `JournalLine.base_amount`.
    pub base_amount: i64,
    /// `JournalLine.fund_id`.
    pub fund_id: String,
    /// `JournalLine.grant_id`.
    pub grant_id: Option<String>,
    /// `JournalLine.dimension_ids`.
    pub dimension_ids: Vec<String>,
    /// `JournalLine.flow_node_id`.
    pub flow_node_id: Option<String>,
    /// `JournalLine.memo`.
    pub memo: Option<String>,
}

/// The latest `run_command` result per caller (`index.ts:142-152`), read through
/// `my_command_result`.
#[spacetimedb::table(accessor = command_result)]
#[derive(Debug, Clone, PartialEq)]
pub struct CommandResult {
    /// The caller.
    #[primary_key]
    pub identity: Identity,
    /// The organisation the command ran on.
    pub org_id: String,
    /// The caller's request id.
    pub request_id: String,
    /// The command's result as JSON.
    pub result_json: String,
    /// When it ran.
    pub at: Timestamp,
}

/// A ledger-core record stored in a doc table: serialised with serde (the TS JSON shape, see
/// `ledger_core::types`) and keyed by its id (`spacetime-codec.ts:203-205`).
pub trait Doc: Serialize + DeserializeOwned {
    /// The record's id, unique within one organisation.
    fn doc_id(&self) -> &str;
}

/// A row of one of the fourteen doc tables (`index.ts:114-125`): `key`, `org_id`, `id`, `seq` and
/// the record as JSON in `data`. The trait lets one loop serve every doc table, as
/// `DocTableHandle` does in TS (`index.ts:351-357`).
pub trait DocRow: Sized {
    /// The ledger-core record this table holds.
    type Doc: Doc;

    /// The table's name, as in `index.ts`.
    const TABLE: &'static str;

    /// The row from its columns.
    fn from_columns(key: String, org_id: String, id: String, seq: u64, data: String) -> Self;

    /// `{org_id}:{id}`.
    fn key(&self) -> &str;

    /// Position in the state's collection.
    fn seq(&self) -> u64;

    /// The record as JSON.
    fn data(&self) -> &str;
}

// The fourteen doc tables (`index.ts:114-140`). Each is spelled out: the table attribute's
// generated code does not survive `macro_rules!` hygiene (its `_table_name` locals stop
// resolving), so only the trait impls below come from a macro.

/// `LedgerState.accounts` (`index.ts:127`).
#[spacetimedb::table(accessor = account)]
#[derive(Debug, Clone, PartialEq)]
pub struct AccountRow {
    /// `{org_id}:{id}`.
    #[primary_key]
    pub key: String,
    /// The organisation.
    #[index(btree)]
    pub org_id: String,
    /// The record's id.
    pub id: String,
    /// Position in the state's collection.
    pub seq: u64,
    /// The record as JSON.
    pub data: String,
}

/// `LedgerState.funds` (`index.ts:128`).
#[spacetimedb::table(accessor = fund)]
#[derive(Debug, Clone, PartialEq)]
pub struct FundRow {
    /// `{org_id}:{id}`.
    #[primary_key]
    pub key: String,
    /// The organisation.
    #[index(btree)]
    pub org_id: String,
    /// The record's id.
    pub id: String,
    /// Position in the state's collection.
    pub seq: u64,
    /// The record as JSON.
    pub data: String,
}

/// `LedgerState.grants` (`index.ts:129`).
#[spacetimedb::table(accessor = grant)]
#[derive(Debug, Clone, PartialEq)]
pub struct GrantRow {
    /// `{org_id}:{id}`.
    #[primary_key]
    pub key: String,
    /// The organisation.
    #[index(btree)]
    pub org_id: String,
    /// The record's id.
    pub id: String,
    /// Position in the state's collection.
    pub seq: u64,
    /// The record as JSON.
    pub data: String,
}

/// `LedgerState.dimensions` (`index.ts:130`).
#[spacetimedb::table(accessor = dimension)]
#[derive(Debug, Clone, PartialEq)]
pub struct DimensionRow {
    /// `{org_id}:{id}`.
    #[primary_key]
    pub key: String,
    /// The organisation.
    #[index(btree)]
    pub org_id: String,
    /// The record's id.
    pub id: String,
    /// Position in the state's collection.
    pub seq: u64,
    /// The record as JSON.
    pub data: String,
}

/// `LedgerState.periods` (`index.ts:131`).
#[spacetimedb::table(accessor = fiscal_period)]
#[derive(Debug, Clone, PartialEq)]
pub struct FiscalPeriodRow {
    /// `{org_id}:{id}`.
    #[primary_key]
    pub key: String,
    /// The organisation.
    #[index(btree)]
    pub org_id: String,
    /// The record's id.
    pub id: String,
    /// Position in the state's collection.
    pub seq: u64,
    /// The record as JSON.
    pub data: String,
}

/// `LedgerState.budgets` (`index.ts:132`).
#[spacetimedb::table(accessor = budget)]
#[derive(Debug, Clone, PartialEq)]
pub struct BudgetRow {
    /// `{org_id}:{id}`.
    #[primary_key]
    pub key: String,
    /// The organisation.
    #[index(btree)]
    pub org_id: String,
    /// The record's id.
    pub id: String,
    /// Position in the state's collection.
    pub seq: u64,
    /// The record as JSON.
    pub data: String,
}

/// `LedgerState.budgetLines` (`index.ts:133`).
#[spacetimedb::table(accessor = budget_line)]
#[derive(Debug, Clone, PartialEq)]
pub struct BudgetLineRow {
    /// `{org_id}:{id}`.
    #[primary_key]
    pub key: String,
    /// The organisation.
    #[index(btree)]
    pub org_id: String,
    /// The record's id.
    pub id: String,
    /// Position in the state's collection.
    pub seq: u64,
    /// The record as JSON.
    pub data: String,
}

/// `LedgerState.vendors` (`index.ts:134`).
#[spacetimedb::table(accessor = vendor)]
#[derive(Debug, Clone, PartialEq)]
pub struct VendorRow {
    /// `{org_id}:{id}`.
    #[primary_key]
    pub key: String,
    /// The organisation.
    #[index(btree)]
    pub org_id: String,
    /// The record's id.
    pub id: String,
    /// Position in the state's collection.
    pub seq: u64,
    /// The record as JSON.
    pub data: String,
}

/// `LedgerState.bills` (`index.ts:135`).
#[spacetimedb::table(accessor = bill)]
#[derive(Debug, Clone, PartialEq)]
pub struct BillRow {
    /// `{org_id}:{id}`.
    #[primary_key]
    pub key: String,
    /// The organisation.
    #[index(btree)]
    pub org_id: String,
    /// The record's id.
    pub id: String,
    /// Position in the state's collection.
    pub seq: u64,
    /// The record as JSON.
    pub data: String,
}

/// `LedgerState.payments` (`index.ts:136`).
#[spacetimedb::table(accessor = payment)]
#[derive(Debug, Clone, PartialEq)]
pub struct PaymentRow {
    /// `{org_id}:{id}`.
    #[primary_key]
    pub key: String,
    /// The organisation.
    #[index(btree)]
    pub org_id: String,
    /// The record's id.
    pub id: String,
    /// Position in the state's collection.
    pub seq: u64,
    /// The record as JSON.
    pub data: String,
}

/// `LedgerState.claims` (`index.ts:137`).
#[spacetimedb::table(accessor = expense_claim)]
#[derive(Debug, Clone, PartialEq)]
pub struct ExpenseClaimRow {
    /// `{org_id}:{id}`.
    #[primary_key]
    pub key: String,
    /// The organisation.
    #[index(btree)]
    pub org_id: String,
    /// The record's id.
    pub id: String,
    /// Position in the state's collection.
    pub seq: u64,
    /// The record as JSON.
    pub data: String,
}

/// `LedgerState.bankAccounts` (`index.ts:138`).
#[spacetimedb::table(accessor = bank_account)]
#[derive(Debug, Clone, PartialEq)]
pub struct BankAccountRow {
    /// `{org_id}:{id}`.
    #[primary_key]
    pub key: String,
    /// The organisation.
    #[index(btree)]
    pub org_id: String,
    /// The record's id.
    pub id: String,
    /// Position in the state's collection.
    pub seq: u64,
    /// The record as JSON.
    pub data: String,
}

/// `LedgerState.statementLines` (`index.ts:139`).
#[spacetimedb::table(accessor = statement_line)]
#[derive(Debug, Clone, PartialEq)]
pub struct StatementLineRow {
    /// `{org_id}:{id}`.
    #[primary_key]
    pub key: String,
    /// The organisation.
    #[index(btree)]
    pub org_id: String,
    /// The record's id.
    pub id: String,
    /// Position in the state's collection.
    pub seq: u64,
    /// The record as JSON.
    pub data: String,
}

/// `LedgerState.audit`, append-only (`index.ts:140`, `index.ts:491-494`).
#[spacetimedb::table(accessor = audit_event)]
#[derive(Debug, Clone, PartialEq)]
pub struct AuditEventRow {
    /// `{org_id}:{id}`.
    #[primary_key]
    pub key: String,
    /// The organisation.
    #[index(btree)]
    pub org_id: String,
    /// The record's id.
    pub id: String,
    /// Position in the state's collection.
    pub seq: u64,
    /// The record as JSON.
    pub data: String,
}

/// Implements [`DocRow`] for a doc table and [`Doc`] for the record it holds.
macro_rules! doc_row {
    ($($table:literal, $row:ident, $record:ty;)+) => {$(
        impl DocRow for $row {
            type Doc = $record;

            const TABLE: &'static str = $table;

            fn from_columns(key: String, org_id: String, id: String, seq: u64, data: String) -> Self {
                Self { key, org_id, id, seq, data }
            }

            fn key(&self) -> &str {
                &self.key
            }

            fn seq(&self) -> u64 {
                self.seq
            }

            fn data(&self) -> &str {
                &self.data
            }
        }

        impl Doc for $record {
            fn doc_id(&self) -> &str {
                self.id.as_str()
            }
        }
    )+};
}

doc_row! {
    "account", AccountRow, Account;
    "fund", FundRow, Fund;
    "grant", GrantRow, Grant;
    "dimension", DimensionRow, DimensionValue;
    "fiscal_period", FiscalPeriodRow, FiscalPeriod;
    "budget", BudgetRow, Budget;
    "budget_line", BudgetLineRow, BudgetLine;
    "vendor", VendorRow, Vendor;
    "bill", BillRow, Bill;
    "payment", PaymentRow, Payment;
    "expense_claim", ExpenseClaimRow, ExpenseClaim;
    "bank_account", BankAccountRow, BankAccount;
    "statement_line", StatementLineRow, StatementLine;
    "audit_event", AuditEventRow, AuditEvent;
}
