//! `LedgerState` to rows and back: the Rust form of `flowtrace/src/lib/ledger/spacetime-codec.ts`.
//!
//! Pure functions over the row structs of [`crate::tables`], so the host tests cover them and
//! `run_command` can load and write back one organisation with them. Journal entries
//! and lines map field by field; every other collection goes through the ledger-core serde shape,
//! which is the TS JSON (`ledger_core::types`).

use std::collections::BTreeMap;

use ledger_core::env::TimestampError;
use ledger_core::types::{
    ActorId, DimensionId, EntryId, FlowNodeId, GrantId, IsoDate, IsoDateError, JournalEntry,
    JournalLine, LedgerState, Minor, OrgLedgerSettings, PeriodId, Timestamp,
};
use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::tables::{
    AccountRow, AuditEventRow, BankAccountRow, BillRow, BudgetLineRow, BudgetRow, DimensionRow,
    Doc, DocRow, ExpenseClaimRow, FiscalPeriodRow, FundRow, GrantRow, JournalEntryRow,
    JournalLineRow, PaymentRow, StatementLineRow, VendorRow,
};

/// Why rows and state did not convert.
#[derive(Debug, thiserror::Error)]
pub enum CodecError {
    /// A record, the settings or the counters did not (de)serialise.
    #[error("{what}: {source}")]
    Json {
        /// Which collection or column.
        what: &'static str,
        /// The serde error.
        source: serde_json::Error,
    },
    /// A string column holds no TS literal of its enum.
    #[error("{value:?} is not a valid {what}.")]
    Literal {
        /// Which column.
        what: &'static str,
        /// The column value.
        value: String,
    },
    /// A date column is not `YYYY-MM-DD`.
    #[error(transparent)]
    Date(#[from] IsoDateError),
    /// A timestamp column is not `toISOString` shaped.
    #[error(transparent)]
    Timestamp(#[from] TimestampError),
    /// An amount outside the JS safe integer range on the way in (`spacetime-codec.ts:110-113`).
    #[error("Amount {0} is not a whole number of minor units.")]
    NotWhole(Minor),
    /// An amount outside the JS safe integer range on the way out (`spacetime-codec.ts:115-119`).
    #[error("Amount {0} is outside the safe integer range.")]
    Unsafe(Minor),
}

/// The largest integer a JS number holds exactly, `Number.MAX_SAFE_INTEGER`.
const MAX_SAFE_INTEGER: Minor = (1 << 53) - 1;

/// `${orgId}:${id}` (`spacetime-codec.ts:104`).
#[must_use]
pub fn row_key(org_id: &str, id: &str) -> String {
    format!("{org_id}:{id}")
}

/// The TS string literal of a ledger-core enum, through its serde names.
fn literal<T: Serialize>(what: &'static str, value: &T) -> Result<String, CodecError> {
    match serde_json::to_value(value) {
        Ok(serde_json::Value::String(s)) => Ok(s),
        Ok(other) => Err(CodecError::Literal {
            what,
            value: other.to_string(),
        }),
        Err(source) => Err(CodecError::Json { what, source }),
    }
}

/// A ledger-core enum from its TS string literal.
fn from_literal<T: DeserializeOwned>(what: &'static str, value: &str) -> Result<T, CodecError> {
    serde_json::from_value(serde_json::Value::String(value.to_owned())).map_err(|_| {
        CodecError::Literal {
            what,
            value: value.to_owned(),
        }
    })
}

/// `toI64` (`spacetime-codec.ts:110-113`): only safe integers go into a row.
fn to_i64(amount: Minor) -> Result<i64, CodecError> {
    if amount.abs() <= MAX_SAFE_INTEGER {
        Ok(amount)
    } else {
        Err(CodecError::NotWhole(amount))
    }
}

/// `fromI64` (`spacetime-codec.ts:115-119`): only safe integers come out of a row.
fn from_i64(amount: i64) -> Result<Minor, CodecError> {
    if amount.abs() <= MAX_SAFE_INTEGER {
        Ok(amount)
    } else {
        Err(CodecError::Unsafe(amount))
    }
}

/// `entryToRow` (`spacetime-codec.ts:121-142`).
///
/// # Errors
///
/// Never in practice: an enum that does not serialise to its TS literal.
pub fn entry_to_row(
    org_id: &str,
    entry: &JournalEntry,
    seq: u64,
) -> Result<JournalEntryRow, CodecError> {
    // Destructured in full so a field added to `JournalEntry` fails to compile here.
    let JournalEntry {
        id,
        number,
        date,
        period_id,
        memo,
        source_type,
        source_id,
        status,
        reversal_of,
        reversed_by,
        override_reason,
        created_by,
        created_at,
        posted_by,
        posted_at,
    } = entry;
    Ok(JournalEntryRow {
        key: row_key(org_id, id.as_str()),
        org_id: org_id.to_owned(),
        id: id.to_string(),
        seq,
        number: number.clone(),
        date: date.to_string(),
        period_id: period_id.to_string(),
        memo: memo.clone(),
        source_type: literal("source type", source_type)?,
        source_id: source_id.clone(),
        status: literal("entry status", status)?,
        reversal_of: reversal_of.as_ref().map(ToString::to_string),
        reversed_by: reversed_by.as_ref().map(ToString::to_string),
        override_reason: override_reason.clone(),
        created_by: created_by.to_string(),
        created_at: created_at.to_string(),
        posted_by: posted_by.as_ref().map(ToString::to_string),
        posted_at: posted_at.as_ref().map(ToString::to_string),
    })
}

/// `rowToEntry` (`spacetime-codec.ts:144-162`), validating what TS only casts.
///
/// # Errors
///
/// A date, timestamp, status or source type the engine types cannot hold.
pub fn row_to_entry(row: &JournalEntryRow) -> Result<JournalEntry, CodecError> {
    Ok(JournalEntry {
        id: EntryId::from(row.id.as_str()),
        number: row.number.clone(),
        date: IsoDate::parse(&row.date)?,
        period_id: PeriodId::from(row.period_id.as_str()),
        memo: row.memo.clone(),
        source_type: from_literal("source type", &row.source_type)?,
        source_id: row.source_id.clone(),
        status: from_literal("entry status", &row.status)?,
        reversal_of: row.reversal_of.as_deref().map(EntryId::from),
        reversed_by: row.reversed_by.as_deref().map(EntryId::from),
        override_reason: row.override_reason.clone(),
        created_by: ActorId::from(row.created_by.as_str()),
        created_at: Timestamp::parse(&row.created_at)?,
        posted_by: row.posted_by.as_deref().map(ActorId::from),
        posted_at: row.posted_at.as_deref().map(Timestamp::parse).transpose()?,
    })
}

/// `lineToRow` (`spacetime-codec.ts:164-183`).
///
/// # Errors
///
/// [`CodecError::NotWhole`] for an amount outside the JS safe integer range (`toI64`).
pub fn line_to_row(
    org_id: &str,
    line: &JournalLine,
    seq: u64,
) -> Result<JournalLineRow, CodecError> {
    // Destructured in full so a field added to `JournalLine` fails to compile here.
    let JournalLine {
        id,
        entry_id,
        account_id,
        debit,
        credit,
        currency,
        fx_rate,
        base_amount,
        fund_id,
        grant_id,
        dimension_ids,
        flow_node_id,
        memo,
    } = line;
    Ok(JournalLineRow {
        key: row_key(org_id, id.as_str()),
        org_id: org_id.to_owned(),
        id: id.to_string(),
        seq,
        entry_id: entry_id.to_string(),
        account_id: account_id.to_string(),
        debit: to_i64(*debit)?,
        credit: to_i64(*credit)?,
        currency: currency.clone(),
        fx_rate: *fx_rate,
        base_amount: to_i64(*base_amount)?,
        fund_id: fund_id.to_string(),
        grant_id: grant_id.as_ref().map(ToString::to_string),
        dimension_ids: dimension_ids.iter().map(ToString::to_string).collect(),
        flow_node_id: flow_node_id.as_ref().map(ToString::to_string),
        memo: memo.clone(),
    })
}

/// `rowToLine` (`spacetime-codec.ts:185-201`).
///
/// # Errors
///
/// [`CodecError::Unsafe`] for an amount outside the JS safe integer range (`fromI64`).
pub fn row_to_line(row: &JournalLineRow) -> Result<JournalLine, CodecError> {
    Ok(JournalLine {
        id: row.id.as_str().into(),
        entry_id: EntryId::from(row.entry_id.as_str()),
        account_id: row.account_id.as_str().into(),
        debit: from_i64(row.debit)?,
        credit: from_i64(row.credit)?,
        currency: row.currency.clone(),
        fx_rate: row.fx_rate,
        base_amount: from_i64(row.base_amount)?,
        fund_id: row.fund_id.as_str().into(),
        grant_id: row.grant_id.as_deref().map(GrantId::from),
        dimension_ids: row
            .dimension_ids
            .iter()
            .map(|d| DimensionId::from(d.as_str()))
            .collect(),
        flow_node_id: row.flow_node_id.as_deref().map(FlowNodeId::from),
        memo: row.memo.clone(),
    })
}

/// `docToRow` (`spacetime-codec.ts:203-205`): the record as JSON, keyed by its id.
///
/// # Errors
///
/// The record does not serialise.
pub fn doc_to_row<R: DocRow>(org_id: &str, doc: &R::Doc, seq: u64) -> Result<R, CodecError> {
    let data = serde_json::to_string(doc).map_err(|source| CodecError::Json {
        what: R::TABLE,
        source,
    })?;
    let id = doc.doc_id();
    Ok(R::from_columns(
        row_key(org_id, id),
        org_id.to_owned(),
        id.to_owned(),
        seq,
        data,
    ))
}

/// The record a doc row holds.
///
/// # Errors
///
/// `data` is not the JSON of the table's record.
pub fn row_to_doc<R: DocRow>(row: &R) -> Result<R::Doc, CodecError> {
    serde_json::from_str(row.data()).map_err(|source| CodecError::Json {
        what: R::TABLE,
        source,
    })
}

/// One organisation's ledger rows, every table but `ledger_org` (`OrgRows`,
/// `spacetime-codec.ts:97-102`). Collections are named as in [`LedgerState`].
#[derive(Default)]
pub struct OrgRows {
    /// `journal_entry` rows.
    pub entries: Vec<JournalEntryRow>,
    /// `journal_line` rows.
    pub lines: Vec<JournalLineRow>,
    /// `account` rows.
    pub accounts: Vec<AccountRow>,
    /// `fund` rows.
    pub funds: Vec<FundRow>,
    /// `grant` rows.
    pub grants: Vec<GrantRow>,
    /// `dimension` rows.
    pub dimensions: Vec<DimensionRow>,
    /// `fiscal_period` rows.
    pub periods: Vec<FiscalPeriodRow>,
    /// `budget` rows.
    pub budgets: Vec<BudgetRow>,
    /// `budget_line` rows.
    pub budget_lines: Vec<BudgetLineRow>,
    /// `vendor` rows.
    pub vendors: Vec<VendorRow>,
    /// `bill` rows.
    pub bills: Vec<BillRow>,
    /// `payment` rows.
    pub payments: Vec<PaymentRow>,
    /// `expense_claim` rows.
    pub claims: Vec<ExpenseClaimRow>,
    /// `bank_account` rows.
    pub bank_accounts: Vec<BankAccountRow>,
    /// `statement_line` rows.
    pub statement_lines: Vec<StatementLineRow>,
    /// `audit_event` rows.
    pub audit: Vec<AuditEventRow>,
}

/// `LedgerOrg.settings_json` for the state's settings (`index.ts:501`).
///
/// # Errors
///
/// The settings do not serialise.
pub fn settings_json(settings: &OrgLedgerSettings) -> Result<String, CodecError> {
    serde_json::to_string(settings).map_err(|source| CodecError::Json {
        what: "settings_json",
        source,
    })
}

/// `LedgerOrg.counters_json` for the state's counters (`index.ts:502`).
///
/// # Errors
///
/// The counters do not serialise.
pub fn counters_json(counters: &BTreeMap<String, u64>) -> Result<String, CodecError> {
    serde_json::to_string(counters).map_err(|source| CodecError::Json {
        what: "counters_json",
        source,
    })
}

/// Rows of one doc collection, numbered on from `next_seq`.
fn docs_to_rows<R: DocRow>(
    org_id: &str,
    docs: &[R::Doc],
    next_seq: &mut u64,
) -> Result<Vec<R>, CodecError> {
    docs.iter()
        .map(|doc| {
            let row = doc_to_row(org_id, doc, *next_seq)?;
            *next_seq += 1;
            Ok(row)
        })
        .collect()
}

/// One doc collection from its rows, in `seq` order (`bySeq`, `spacetime-codec.ts:207-208`).
fn rows_to_docs<R: DocRow>(rows: &[R]) -> Result<Vec<R::Doc>, CodecError> {
    let mut sorted: Vec<&R> = rows.iter().collect();
    sorted.sort_by_key(|r| r.seq());
    sorted.into_iter().map(row_to_doc).collect()
}

/// Every row of `state`, numbered from `next_seq` on in the order `writeBack` inserts a new
/// organisation's rows: entries, lines, then the doc collections in `DOC_COLLECTIONS` order
/// (`index.ts:465-497`, `spacetime-codec.ts:21-36`). Leaves `next_seq` at the next free `seq`.
///
/// # Errors
///
/// A record that does not convert.
pub fn state_to_rows(
    org_id: &str,
    state: &LedgerState,
    next_seq: &mut u64,
) -> Result<OrgRows, CodecError> {
    let mut numbered = || {
        let seq = *next_seq;
        *next_seq += 1;
        seq
    };
    let entries = state
        .entries
        .iter()
        .map(|e| entry_to_row(org_id, e, numbered()))
        .collect::<Result<_, _>>()?;
    let lines = state
        .lines
        .iter()
        .map(|l| line_to_row(org_id, l, numbered()))
        .collect::<Result<_, _>>()?;
    Ok(OrgRows {
        entries,
        lines,
        accounts: docs_to_rows(org_id, &state.accounts, next_seq)?,
        funds: docs_to_rows(org_id, &state.funds, next_seq)?,
        grants: docs_to_rows(org_id, &state.grants, next_seq)?,
        dimensions: docs_to_rows(org_id, &state.dimensions, next_seq)?,
        periods: docs_to_rows(org_id, &state.periods, next_seq)?,
        budgets: docs_to_rows(org_id, &state.budgets, next_seq)?,
        budget_lines: docs_to_rows(org_id, &state.budget_lines, next_seq)?,
        vendors: docs_to_rows(org_id, &state.vendors, next_seq)?,
        bills: docs_to_rows(org_id, &state.bills, next_seq)?,
        payments: docs_to_rows(org_id, &state.payments, next_seq)?,
        claims: docs_to_rows(org_id, &state.claims, next_seq)?,
        bank_accounts: docs_to_rows(org_id, &state.bank_accounts, next_seq)?,
        statement_lines: docs_to_rows(org_id, &state.statement_lines, next_seq)?,
        audit: docs_to_rows(org_id, &state.audit, next_seq)?,
    })
}

/// `assembleState` (`spacetime-codec.ts:216-243`): the engine's state from one organisation's
/// rows, each collection in `seq` order.
///
/// # Errors
///
/// A row, the settings or the counters that do not convert.
pub fn assemble_state(
    settings_json: &str,
    counters_json: &str,
    rows: &OrgRows,
) -> Result<LedgerState, CodecError> {
    let mut entries: Vec<&JournalEntryRow> = rows.entries.iter().collect();
    entries.sort_by_key(|r| r.seq);
    let mut lines: Vec<&JournalLineRow> = rows.lines.iter().collect();
    lines.sort_by_key(|r| r.seq);
    Ok(LedgerState {
        settings: serde_json::from_str(settings_json).map_err(|source| CodecError::Json {
            what: "settings_json",
            source,
        })?,
        accounts: rows_to_docs(&rows.accounts)?,
        funds: rows_to_docs(&rows.funds)?,
        grants: rows_to_docs(&rows.grants)?,
        dimensions: rows_to_docs(&rows.dimensions)?,
        periods: rows_to_docs(&rows.periods)?,
        entries: entries
            .into_iter()
            .map(row_to_entry)
            .collect::<Result<_, _>>()?,
        lines: lines
            .into_iter()
            .map(row_to_line)
            .collect::<Result<_, _>>()?,
        budgets: rows_to_docs(&rows.budgets)?,
        budget_lines: rows_to_docs(&rows.budget_lines)?,
        vendors: rows_to_docs(&rows.vendors)?,
        bills: rows_to_docs(&rows.bills)?,
        payments: rows_to_docs(&rows.payments)?,
        claims: rows_to_docs(&rows.claims)?,
        bank_accounts: rows_to_docs(&rows.bank_accounts)?,
        statement_lines: rows_to_docs(&rows.statement_lines)?,
        audit: rows_to_docs(&rows.audit)?,
        counters: serde_json::from_str(counters_json).map_err(|source| CodecError::Json {
            what: "counters_json",
            source,
        })?,
    })
}

#[cfg(test)]
mod tests {
    #![expect(clippy::unwrap_used, reason = "tests may unwrap")]

    use super::*;

    use ledger_core::types::{Account, EntryStatus, SourceType};

    /// `rust/fixtures/demo-books.json#/state`, the demo organisation's books.
    fn demo_state() -> LedgerState {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../fixtures/demo-books.json"
        );
        let fixture: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
        serde_json::from_value(fixture["state"].clone()).unwrap()
    }

    fn first_posted_entry(state: &LedgerState) -> JournalEntry {
        state
            .entries
            .iter()
            .find(|e| e.status == EntryStatus::Posted)
            .unwrap()
            .clone()
    }

    #[test]
    fn row_key_joins_org_and_id_with_a_colon() {
        assert_eq!(row_key("org-1", "acc-1110"), "org-1:acc-1110");
    }

    #[test]
    fn demo_books_survive_rows_and_back() {
        let state = demo_state();
        let mut next_seq = 0;
        let rows = state_to_rows("demo", &state, &mut next_seq).unwrap();
        let settings = settings_json(&state.settings).unwrap();
        let counters = counters_json(&state.counters).unwrap();
        assert_eq!(assemble_state(&settings, &counters, &rows).unwrap(), state);
    }

    #[test]
    fn state_to_rows_numbers_every_row_once_from_next_seq() {
        let state = demo_state();
        let mut next_seq = 7;
        let rows = state_to_rows("demo", &state, &mut next_seq).unwrap();
        // 306 records in the demo books (rust/fixtures/README.md, demo-books.json).
        assert_eq!(
            (
                rows.entries[0].seq,
                rows.lines[0].seq,
                rows.accounts[0].seq,
                next_seq
            ),
            (7, 7 + 22, 7 + 22 + 49, 7 + 306)
        );
    }

    #[test]
    fn assemble_state_orders_each_collection_by_seq() {
        let state = demo_state();
        let mut next_seq = 0;
        let mut rows = state_to_rows("demo", &state, &mut next_seq).unwrap();
        rows.entries.reverse();
        rows.accounts.reverse();
        let settings = settings_json(&state.settings).unwrap();
        let counters = counters_json(&state.counters).unwrap();
        let back = assemble_state(&settings, &counters, &rows).unwrap();
        assert_eq!(
            (back.entries, back.accounts),
            (state.entries, state.accounts)
        );
    }

    #[test]
    fn entry_rows_carry_the_ts_literals_and_the_composite_key() {
        let entry = first_posted_entry(&demo_state());
        let row = entry_to_row("demo", &entry, 3).unwrap();
        assert_eq!(
            (row.key.as_str(), row.status.as_str(), row.seq),
            (format!("demo:{}", entry.id).as_str(), "posted", 3)
        );
    }

    #[test]
    fn entry_source_type_is_written_snake_case() {
        let mut entry = first_posted_entry(&demo_state());
        entry.source_type = SourceType::ExpenseClaim;
        assert_eq!(
            entry_to_row("demo", &entry, 0).unwrap().source_type,
            "expense_claim"
        );
    }

    #[test]
    fn row_to_entry_refuses_a_status_the_engine_does_not_know() {
        let entry = first_posted_entry(&demo_state());
        let mut row = entry_to_row("demo", &entry, 0).unwrap();
        row.status = "open".to_owned();
        assert_eq!(
            row_to_entry(&row).unwrap_err().to_string(),
            "\"open\" is not a valid entry status."
        );
    }

    #[test]
    fn row_to_entry_refuses_a_date_that_is_not_iso() {
        let entry = first_posted_entry(&demo_state());
        let mut row = entry_to_row("demo", &entry, 0).unwrap();
        row.date = "04/02/2026".to_owned();
        assert!(matches!(row_to_entry(&row), Err(CodecError::Date(_))));
    }

    #[test]
    fn line_to_row_refuses_an_amount_past_the_safe_integer_range() {
        let mut line = demo_state().lines[0].clone();
        line.debit = 9_007_199_254_740_992;
        assert_eq!(
            line_to_row("demo", &line, 0).unwrap_err().to_string(),
            "Amount 9007199254740992 is not a whole number of minor units."
        );
    }

    #[test]
    fn row_to_line_refuses_an_amount_past_the_safe_integer_range() {
        let line = demo_state().lines[0].clone();
        let mut row = line_to_row("demo", &line, 0).unwrap();
        row.base_amount = -9_007_199_254_740_992;
        assert_eq!(
            row_to_line(&row).unwrap_err().to_string(),
            "Amount -9007199254740992 is outside the safe integer range."
        );
    }

    #[test]
    fn line_rows_keep_the_largest_safe_amount() {
        let mut line = demo_state().lines[0].clone();
        line.credit = 9_007_199_254_740_991;
        let row = line_to_row("demo", &line, 0).unwrap();
        assert_eq!(row_to_line(&row).unwrap(), line);
    }

    #[test]
    fn doc_rows_hold_the_record_json_under_its_id() {
        let account = demo_state().accounts[0].clone();
        let row: AccountRow = doc_to_row("demo", &account, 5).unwrap();
        assert_eq!(
            (row.key, row.id, row.seq, row.data),
            (
                format!("demo:{}", account.id),
                account.id.to_string(),
                5,
                serde_json::to_string(&account).unwrap()
            )
        );
    }

    #[test]
    fn row_to_doc_refuses_data_of_another_shape() {
        let account: Account = demo_state().accounts[0].clone();
        let mut row: AccountRow = doc_to_row("demo", &account, 0).unwrap();
        row.data = r#"{"id":"acc-1"}"#.to_owned();
        assert!(matches!(
            row_to_doc(&row),
            Err(CodecError::Json {
                what: "account",
                ..
            })
        ));
    }
}
