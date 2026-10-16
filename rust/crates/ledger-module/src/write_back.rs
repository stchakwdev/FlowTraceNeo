//! What a command changes in the tables: `writeBack` (`spacetimedb/src/index.ts:428-506`) as a
//! pure diff of the books before and after the engine ran, and the rows `import_books` writes.
//!
//! [`plan`] compares every record with the record loaded from its row, as TS compares
//! `JSON.stringify` of both (`index.ts:449-450`): new ids are inserted with fresh `seq` numbers,
//! changed records are updated in place under their old `seq`, and ids that are gone are deleted.
//! On top of the engine's rules it refuses to rewrite history (`index.ts:12-14`): posted entries
//! change only by being reversed, lines of posted entries never change, and the audit log is
//! append-only. The reducer then applies the planned operations table by table.

use std::collections::{HashMap, HashSet};

use ledger_core::types::{EntryStatus, JournalEntry, JournalLine, LedgerState};
use serde::Serialize;

use crate::codec::{
    CodecError, OrgRows, counters_json, doc_to_row, entry_to_row, line_to_row, row_key,
    settings_json, state_to_rows,
};
use crate::tables::{
    AccountRow, AuditEventRow, BankAccountRow, BillRow, BudgetLineRow, BudgetRow, DimensionRow,
    Doc, DocRow, ExpenseClaimRow, FiscalPeriodRow, FundRow, GrantRow, JournalEntryRow,
    JournalLineRow, PaymentRow, StatementLineRow, VendorRow,
};

/// The writes to one table (`sync`'s `insert`, `update` and `remove`, `index.ts:430-462`).
#[derive(Debug, Clone, PartialEq)]
pub struct TableOps<R> {
    /// New rows, each with a fresh `seq`.
    pub insert: Vec<R>,
    /// Changed rows, each under the `seq` it had.
    pub update: Vec<R>,
    /// Keys of rows whose record is gone.
    pub delete: Vec<String>,
}

impl<R> Default for TableOps<R> {
    fn default() -> Self {
        Self {
            insert: Vec::new(),
            update: Vec::new(),
            delete: Vec::new(),
        }
    }
}

impl<R> TableOps<R> {
    /// How many rows the table writes.
    #[must_use]
    pub fn len(&self) -> usize {
        self.insert.len() + self.update.len() + self.delete.len()
    }

    /// True when the table is left as it is.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// Everything `writeBack` writes for one command: the row changes of each table, named as the
/// [`LedgerState`] collections, and the new `ledger_org` columns (`index.ts:500-505`).
#[derive(Debug, Clone, PartialEq)]
pub struct WriteBack {
    /// `journal_entry`.
    pub entries: TableOps<JournalEntryRow>,
    /// `journal_line`.
    pub lines: TableOps<JournalLineRow>,
    /// `account`.
    pub accounts: TableOps<AccountRow>,
    /// `fund`.
    pub funds: TableOps<FundRow>,
    /// `grant`.
    pub grants: TableOps<GrantRow>,
    /// `dimension`.
    pub dimensions: TableOps<DimensionRow>,
    /// `fiscal_period`.
    pub periods: TableOps<FiscalPeriodRow>,
    /// `budget`.
    pub budgets: TableOps<BudgetRow>,
    /// `budget_line`.
    pub budget_lines: TableOps<BudgetLineRow>,
    /// `vendor`.
    pub vendors: TableOps<VendorRow>,
    /// `bill`.
    pub bills: TableOps<BillRow>,
    /// `payment`.
    pub payments: TableOps<PaymentRow>,
    /// `expense_claim`.
    pub claims: TableOps<ExpenseClaimRow>,
    /// `bank_account`.
    pub bank_accounts: TableOps<BankAccountRow>,
    /// `statement_line`.
    pub statement_lines: TableOps<StatementLineRow>,
    /// `audit_event`.
    pub audit: TableOps<AuditEventRow>,
    /// `ledger_org.settings_json`.
    pub settings_json: String,
    /// `ledger_org.counters_json`.
    pub counters_json: String,
    /// `ledger_org.next_seq`.
    pub next_seq: u64,
}

/// The TS text of a codec error, which TS throws as a plain `Error` (`spacetime-codec.ts:110-119`).
fn text(e: impl std::fmt::Display) -> String {
    e.to_string()
}

/// `JSON.stringify(item)`, the form two records are compared in.
fn json<T: Serialize>(item: &T) -> Result<String, String> {
    serde_json::to_string(item).map_err(|e| e.to_string())
}

/// A record as loaded (`loadOrg`'s `prior`, `index.ts:398-406`).
struct Prior<'a, T> {
    key: String,
    json: String,
    seq: u64,
    item: &'a T,
}

/// The running `nextSeq` of one write-back.
struct Sync<'a> {
    org_id: &'a str,
    next_seq: u64,
}

impl Sync<'_> {
    /// `sync(name, items, ops)` (`index.ts:430-462`) for one collection: `seqs` holds the `seq`
    /// of every loaded row by key; `guard` sees the loaded record and its replacement, or `None`
    /// when it is gone.
    fn collection<T: Serialize, R>(
        &mut self,
        name: &str,
        (before, after): (&[T], &[T]),
        seqs: &HashMap<&str, u64>,
        id: fn(&T) -> &str,
        to_row: impl Fn(&str, &T, u64) -> Result<R, CodecError>,
        guard: impl Fn(&T, Option<&T>) -> Result<(), String>,
    ) -> Result<TableOps<R>, String> {
        // A `Map` keyed by row key in load order; a repeated key keeps its first place and takes
        // the later record, as `Map.prototype.set` does.
        let mut prior: Vec<Prior<'_, T>> = Vec::with_capacity(before.len());
        let mut place: HashMap<String, usize> = HashMap::with_capacity(before.len());
        for item in before {
            let key = row_key(self.org_id, id(item));
            let loaded = Prior {
                json: json(item)?,
                seq: seqs.get(key.as_str()).copied().unwrap_or(0),
                item,
                key: key.clone(),
            };
            if let Some(&i) = place.get(&key) {
                prior[i] = loaded;
            } else {
                place.insert(key, prior.len());
                prior.push(loaded);
            }
        }
        let mut ops = TableOps::default();
        let mut seen: HashSet<String> = HashSet::with_capacity(after.len());
        for item in after {
            let key = row_key(self.org_id, id(item));
            if !seen.insert(key.clone()) {
                return Err(format!("Duplicate id {} in {name}.", id(item)));
            }
            let Some(prev) = place.get(&key).map(|&i| &prior[i]) else {
                ops.insert
                    .push(to_row(self.org_id, item, self.next_seq).map_err(text)?);
                self.next_seq += 1;
                continue;
            };
            if json(item)? == prev.json {
                continue;
            }
            guard(prev.item, Some(item))?;
            ops.update
                .push(to_row(self.org_id, item, prev.seq).map_err(text)?);
        }
        for prev in &prior {
            if seen.contains(&prev.key) {
                continue;
            }
            guard(prev.item, None)?;
            ops.delete.push(prev.key.clone());
        }
        Ok(ops)
    }

    /// The journal tables (`index.ts:464-479`): entries under [`guard_entry`], then lines, which
    /// may only change while their entry was a draft when loaded.
    fn journal(
        &mut self,
        rows: &OrgRows,
        before: &LedgerState,
        after: &LedgerState,
    ) -> Result<(TableOps<JournalEntryRow>, TableOps<JournalLineRow>), String> {
        let entry_seqs: HashMap<&str, u64> = rows
            .entries
            .iter()
            .map(|r| (r.key.as_str(), r.seq))
            .collect();
        let entries = self.collection(
            "entries",
            (&before.entries, &after.entries),
            &entry_seqs,
            |e| e.id.as_str(),
            entry_to_row,
            guard_entry,
        )?;

        // `priorStatus` (`index.ts:407`): the status of every entry as loaded.
        let prior_status: HashMap<&str, EntryStatus> = before
            .entries
            .iter()
            .map(|e| (e.id.as_str(), e.status))
            .collect();
        let line_seqs: HashMap<&str, u64> =
            rows.lines.iter().map(|r| (r.key.as_str(), r.seq)).collect();
        let lines = self.collection(
            "lines",
            (&before.lines, &after.lines),
            &line_seqs,
            |l| l.id.as_str(),
            line_to_row,
            |prev: &JournalLine, _| match prior_status.get(prev.entry_id.as_str()) {
                Some(&status) if status != EntryStatus::Draft => {
                    Err(format!("Line {} of a posted entry cannot change.", prev.id))
                }
                _ => Ok(()),
            },
        )?;
        Ok((entries, lines))
    }

    /// [`Sync::collection`] for a doc table.
    fn docs<R: DocRow>(
        &mut self,
        name: &str,
        (before, after): (&[R::Doc], &[R::Doc]),
        rows: &[R],
        guard: impl Fn(&R::Doc, Option<&R::Doc>) -> Result<(), String>,
    ) -> Result<TableOps<R>, String> {
        let seqs: HashMap<&str, u64> = rows.iter().map(|r| (r.key(), r.seq())).collect();
        self.collection(
            name,
            (before, after),
            &seqs,
            <R::Doc as Doc>::doc_id,
            |org_id, doc, seq| doc_to_row::<R>(org_id, doc, seq),
            guard,
        )
    }
}

/// A doc collection with no history rule.
#[expect(
    clippy::unnecessary_wraps,
    reason = "one guard signature for every collection"
)]
fn unguarded<T>(_: &T, _: Option<&T>) -> Result<(), String> {
    Ok(())
}

/// `guard: () => throw new Error('The audit log is append-only.')` (`index.ts:491-495`).
fn append_only<T>(_: &T, _: Option<&T>) -> Result<(), String> {
    Err("The audit log is append-only.".to_owned())
}

/// The TS literal of an entry status.
fn status_literal(status: EntryStatus) -> &'static str {
    match status {
        EntryStatus::Draft => "draft",
        EntryStatus::Posted => "posted",
        EntryStatus::Reversed => "reversed",
    }
}

/// `guardEntry` (`index.ts:412-426`): a draft may change or go; a posted entry changes only
/// its `status` and `reversed_by`, and only from posted to reversed.
fn guard_entry(prev: &JournalEntry, next: Option<&JournalEntry>) -> Result<(), String> {
    if prev.status == EntryStatus::Draft {
        return Ok(());
    }
    let Some(next) = next else {
        return Err(format!("Posted entry {} cannot be deleted.", prev.number));
    };
    // `Object.keys(prev)` in `rowToEntry` order (`spacetime-codec.ts:144-162`), less the two
    // keys a reversal sets. Destructured in full so a new field fails to compile here.
    let JournalEntry {
        id,
        number,
        date,
        period_id,
        memo,
        source_type,
        source_id,
        status: _,
        reversal_of,
        reversed_by: _,
        override_reason,
        created_by,
        created_at,
        posted_by,
        posted_at,
    } = prev;
    let changed = [
        ("id", *id != next.id),
        ("number", *number != next.number),
        ("date", *date != next.date),
        ("period_id", *period_id != next.period_id),
        ("memo", *memo != next.memo),
        ("source_type", *source_type != next.source_type),
        ("source_id", *source_id != next.source_id),
        ("reversal_of", *reversal_of != next.reversal_of),
        ("override_reason", *override_reason != next.override_reason),
        ("created_by", *created_by != next.created_by),
        ("created_at", *created_at != next.created_at),
        ("posted_by", *posted_by != next.posted_by),
        ("posted_at", *posted_at != next.posted_at),
    ];
    if let Some((key, _)) = changed.iter().find(|(_, differs)| *differs) {
        return Err(format!(
            "Posted entry {} cannot change ({key}).",
            prev.number
        ));
    }
    let reversal = prev.status == EntryStatus::Posted && next.status == EntryStatus::Reversed;
    if prev.status != next.status && !reversal {
        return Err(format!(
            "Entry {} cannot go from {} to {}.",
            prev.number,
            status_literal(prev.status),
            status_literal(next.status)
        ));
    }
    Ok(())
}

/// `writeBack(ctx, orgId, loaded, state)` (`index.ts:428-506`) as a plan: `rows` and `next_seq`
/// are what `run_command` loaded, `before` the books assembled from them and `after` the books
/// once the engine ran. Collections are diffed in the TS order (entries, lines, then
/// `DOC_COLLECTION_KEYS`, `spacetime-codec.ts:21-38`), which fixes both the `seq` of new rows and
/// which refusal wins when several apply.
///
/// # Errors
///
/// The TS texts: `Settings belong to another organisation.`, `Duplicate id <id> in <name>.`, the
/// history guards, and a codec error for an amount outside the safe integer range.
pub fn plan(
    org_id: &str,
    rows: &OrgRows,
    next_seq: u64,
    before: &LedgerState,
    after: &LedgerState,
) -> Result<WriteBack, String> {
    if after.settings.organization_id.as_str() != org_id {
        return Err("Settings belong to another organisation.".to_owned());
    }
    let mut sync = Sync { org_id, next_seq };

    let (entries, lines) = sync.journal(rows, before, after)?;
    let b = before;
    let a = after;
    Ok(WriteBack {
        entries,
        lines,
        accounts: sync.docs(
            "accounts",
            (&b.accounts, &a.accounts),
            &rows.accounts,
            unguarded,
        )?,
        funds: sync.docs("funds", (&b.funds, &a.funds), &rows.funds, unguarded)?,
        grants: sync.docs("grants", (&b.grants, &a.grants), &rows.grants, unguarded)?,
        dimensions: sync.docs(
            "dimensions",
            (&b.dimensions, &a.dimensions),
            &rows.dimensions,
            unguarded,
        )?,
        periods: sync.docs(
            "periods",
            (&b.periods, &a.periods),
            &rows.periods,
            unguarded,
        )?,
        budgets: sync.docs(
            "budgets",
            (&b.budgets, &a.budgets),
            &rows.budgets,
            unguarded,
        )?,
        budget_lines: sync.docs(
            "budgetLines",
            (&b.budget_lines, &a.budget_lines),
            &rows.budget_lines,
            unguarded,
        )?,
        vendors: sync.docs(
            "vendors",
            (&b.vendors, &a.vendors),
            &rows.vendors,
            unguarded,
        )?,
        bills: sync.docs("bills", (&b.bills, &a.bills), &rows.bills, unguarded)?,
        payments: sync.docs(
            "payments",
            (&b.payments, &a.payments),
            &rows.payments,
            unguarded,
        )?,
        claims: sync.docs("claims", (&b.claims, &a.claims), &rows.claims, unguarded)?,
        bank_accounts: sync.docs(
            "bankAccounts",
            (&b.bank_accounts, &a.bank_accounts),
            &rows.bank_accounts,
            unguarded,
        )?,
        statement_lines: sync.docs(
            "statementLines",
            (&b.statement_lines, &a.statement_lines),
            &rows.statement_lines,
            unguarded,
        )?,
        audit: sync.docs("audit", (&b.audit, &a.audit), &rows.audit, append_only)?,
        settings_json: settings_json(&a.settings).map_err(text)?,
        counters_json: counters_json(&a.counters).map_err(text)?,
        next_seq: sync.next_seq,
    })
}

/// Every row of a whole set of books: what `create_org_ledger` writes for new books and
/// `import_books` writes in place of an organisation's books.
pub struct NewRows {
    /// Every ledger row of the books.
    pub rows: OrgRows,
    /// `ledger_org.settings_json`.
    pub settings_json: String,
    /// `ledger_org.counters_json`.
    pub counters_json: String,
    /// `ledger_org.next_seq` after the rows.
    pub next_seq: u64,
}

/// The books as rows of `org_id`, numbered on from `next_seq`, in the order `writeBack` inserts
/// a new organisation's rows ([`state_to_rows`], `index.ts:571-572`).
///
/// # Errors
///
/// The settings name another organisation (the `writeBack` check, `index.ts:429`); a record
/// does not convert to its row.
pub fn books_to_rows(org_id: &str, books: &LedgerState, next_seq: u64) -> Result<NewRows, String> {
    if books.settings.organization_id.as_str() != org_id {
        return Err("Settings belong to another organisation.".to_owned());
    }
    let mut next_seq = next_seq;
    let rows = state_to_rows(org_id, books, &mut next_seq).map_err(text)?;
    Ok(NewRows {
        rows,
        settings_json: settings_json(&books.settings).map_err(text)?,
        counters_json: counters_json(&books.counters).map_err(text)?,
        next_seq,
    })
}

/// The books in `state_json` as rows of `org_id` ([`books_to_rows`]), numbered on from the
/// organisation's `next_seq` so no `seq` is used twice. No TS counterpart: TS loads demo books in
/// the browser only.
///
/// # Errors
///
/// `state_json` is not a `LedgerState`, or [`books_to_rows`] refuses it.
pub fn import(org_id: &str, state_json: &str, next_seq: u64) -> Result<NewRows, String> {
    let books: LedgerState = serde_json::from_str(state_json)
        .map_err(|e| format!("The books are not a valid ledger state: {e}."))?;
    books_to_rows(org_id, &books, next_seq)
}

#[cfg(test)]
mod tests {
    #![expect(clippy::unwrap_used, reason = "tests may unwrap")]

    use super::*;

    use ledger_core::env::fixed;
    use ledger_core::types::{Actor, ActorId, AuditEvent};

    use crate::codec::assemble_state;
    use crate::command::{apply, parse_command};

    const ORG: &str = "550e8400-e29b-41d4-a716-446655440000";

    /// `rust/fixtures/demo-books.json#/state`, whose organisation is [`ORG`].
    fn demo_state() -> LedgerState {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../fixtures/demo-books.json"
        );
        let fixture: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
        serde_json::from_value(fixture["state"].clone()).unwrap()
    }

    /// The demo books as rows from `seq` 0, and the books read back from them.
    fn loaded() -> (OrgRows, u64, LedgerState) {
        let state = demo_state();
        let mut next_seq = 0;
        let rows = state_to_rows(ORG, &state, &mut next_seq).unwrap();
        let settings = settings_json(&state.settings).unwrap();
        let counters = counters_json(&state.counters).unwrap();
        let before = assemble_state(&settings, &counters, &rows).unwrap();
        (rows, next_seq, before)
    }

    fn admin() -> Actor {
        Actor {
            id: ActorId::from("user_dummy"),
            name: "Demo User".to_owned(),
            can_post: true,
            can_approve: true,
            can_close: true,
        }
    }

    /// `before` after `command` ran with the test env.
    fn run(before: &LedgerState, command: &str) -> LedgerState {
        let mut after = before.clone();
        let mut env = fixed::test();
        apply(
            &mut after,
            &mut env,
            &admin(),
            parse_command(command).unwrap(),
        )
        .unwrap();
        after
    }

    fn posted_index(state: &LedgerState) -> usize {
        state
            .entries
            .iter()
            .position(|e| e.status == EntryStatus::Posted)
            .unwrap()
    }

    const DRAFT: &str = r#"{"type":"entry.saveDraft","input":{"date":"2026-02-04","memo":"Paper","lines":[
        {"account_id":"acc-7100","debit":250000,"fund_id":"fund-unrestricted"},
        {"account_id":"acc-1110","credit":250000,"fund_id":"fund-unrestricted"}]}}"#;

    #[test]
    fn unchanged_books_write_nothing_but_the_org_columns() {
        let (rows, next_seq, before) = loaded();
        let wb = plan(ORG, &rows, next_seq, &before, &before).unwrap();
        let total = wb.entries.len()
            + wb.lines.len()
            + wb.accounts.len()
            + wb.periods.len()
            + wb.audit.len();
        assert_eq!(
            (total, wb.next_seq, wb.settings_json, wb.counters_json),
            (
                0,
                next_seq,
                settings_json(&before.settings).unwrap(),
                counters_json(&before.counters).unwrap()
            )
        );
    }

    #[test]
    fn a_new_draft_inserts_its_entry_then_its_lines_then_its_audit_event_in_seq_order() {
        let (rows, next_seq, before) = loaded();
        let after = run(&before, DRAFT);
        let wb = plan(ORG, &rows, next_seq, &before, &after).unwrap();
        let seqs = |ops: &[u64]| ops.to_vec();
        assert_eq!(
            (
                seqs(&wb.entries.insert.iter().map(|r| r.seq).collect::<Vec<_>>()),
                seqs(&wb.lines.insert.iter().map(|r| r.seq).collect::<Vec<_>>()),
                seqs(&wb.audit.insert.iter().map(|r| r.seq).collect::<Vec<_>>()),
                wb.next_seq,
            ),
            (
                vec![next_seq],
                vec![next_seq + 1, next_seq + 2],
                vec![next_seq + 3],
                next_seq + 4
            )
        );
        assert_eq!(wb.entries.insert[0].key, format!("{ORG}:id-1"));
        assert_eq!(wb.counters_json, counters_json(&after.counters).unwrap());
    }

    #[test]
    fn a_changed_record_is_updated_under_the_seq_it_had() {
        let (rows, next_seq, before) = loaded();
        let mut after = before.clone();
        after.accounts[3].name = "Renamed".to_owned();
        let wb = plan(ORG, &rows, next_seq, &before, &after).unwrap();
        let row = &wb.accounts.update[0];
        assert_eq!(
            (
                wb.accounts.len(),
                row.key.clone(),
                row.seq,
                row.data.clone(),
                wb.next_seq
            ),
            (
                1,
                rows.accounts[3].key.clone(),
                rows.accounts[3].seq,
                serde_json::to_string(&after.accounts[3]).unwrap(),
                next_seq
            )
        );
    }

    #[test]
    fn a_deleted_draft_deletes_its_entry_and_lines_by_key() {
        let (rows, next_seq, before) = loaded();
        let with_draft = run(&before, DRAFT);
        let wb = plan(ORG, &rows, next_seq, &before, &with_draft).unwrap();
        let mut rows = rows;
        rows.entries.extend(wb.entries.insert);
        rows.lines.extend(wb.lines.insert);
        rows.audit.extend(wb.audit.insert);
        let mut gone = with_draft.clone();
        gone.entries.pop();
        gone.lines.truncate(gone.lines.len() - 2);
        let wb = plan(ORG, &rows, wb.next_seq, &with_draft, &gone).unwrap();
        assert_eq!(
            (wb.entries.delete, wb.lines.delete),
            (
                vec![format!("{ORG}:id-1")],
                vec![format!("{ORG}:id-2"), format!("{ORG}:id-3")]
            )
        );
    }

    #[test]
    fn a_posted_entry_cannot_be_deleted() {
        let (rows, next_seq, before) = loaded();
        let mut after = before.clone();
        let i = posted_index(&before);
        after.entries.remove(i);
        assert_eq!(
            plan(ORG, &rows, next_seq, &before, &after).unwrap_err(),
            format!(
                "Posted entry {} cannot be deleted.",
                before.entries[i].number
            )
        );
    }

    #[test]
    fn a_posted_entry_cannot_change_its_memo() {
        let (rows, next_seq, before) = loaded();
        let mut after = before.clone();
        let i = posted_index(&before);
        after.entries[i].memo = "Rewritten".to_owned();
        assert_eq!(
            plan(ORG, &rows, next_seq, &before, &after).unwrap_err(),
            format!(
                "Posted entry {} cannot change (memo).",
                before.entries[i].number
            )
        );
    }

    #[test]
    fn the_first_changed_key_in_ts_order_is_named() {
        let (rows, next_seq, before) = loaded();
        let mut after = before.clone();
        let i = posted_index(&before);
        after.entries[i].memo = "Rewritten".to_owned();
        after.entries[i].date = ledger_core::types::IsoDate::parse("2026-01-01").unwrap();
        assert!(
            plan(ORG, &rows, next_seq, &before, &after)
                .unwrap_err()
                .ends_with("cannot change (date).")
        );
    }

    #[test]
    fn a_posted_entry_cannot_go_back_to_draft() {
        let (rows, next_seq, before) = loaded();
        let mut after = before.clone();
        let i = posted_index(&before);
        after.entries[i].status = EntryStatus::Draft;
        assert_eq!(
            plan(ORG, &rows, next_seq, &before, &after).unwrap_err(),
            format!(
                "Entry {} cannot go from posted to draft.",
                before.entries[i].number
            )
        );
    }

    #[test]
    fn a_posted_entry_may_be_marked_reversed() {
        let (rows, next_seq, before) = loaded();
        let mut after = before.clone();
        let i = posted_index(&before);
        after.entries[i].status = EntryStatus::Reversed;
        after.entries[i].reversed_by = Some("je-new".into());
        let wb = plan(ORG, &rows, next_seq, &before, &after).unwrap();
        assert_eq!(wb.entries.update.len(), 1);
    }

    #[test]
    fn a_line_of_a_posted_entry_cannot_change() {
        let (rows, next_seq, before) = loaded();
        let posted = &before.entries[posted_index(&before)];
        let j = before
            .lines
            .iter()
            .position(|l| l.entry_id == posted.id)
            .unwrap();
        let mut after = before.clone();
        after.lines[j].memo = Some("Rewritten".to_owned());
        assert_eq!(
            plan(ORG, &rows, next_seq, &before, &after).unwrap_err(),
            format!(
                "Line {} of a posted entry cannot change.",
                before.lines[j].id
            )
        );
    }

    #[test]
    fn an_audit_event_cannot_change() {
        let (rows, next_seq, before) = loaded();
        let mut after = before.clone();
        after.audit[0].detail = "Rewritten".to_owned();
        assert_eq!(
            plan(ORG, &rows, next_seq, &before, &after).unwrap_err(),
            "The audit log is append-only."
        );
    }

    #[test]
    fn an_audit_event_cannot_be_deleted() {
        let (rows, next_seq, before) = loaded();
        let mut after = before.clone();
        let _: AuditEvent = after.audit.remove(0);
        assert_eq!(
            plan(ORG, &rows, next_seq, &before, &after).unwrap_err(),
            "The audit log is append-only."
        );
    }

    #[test]
    fn an_id_twice_in_one_collection_is_refused_with_the_ts_collection_name() {
        let (rows, next_seq, before) = loaded();
        let mut after = before.clone();
        after.budget_lines.push(after.budget_lines[0].clone());
        assert_eq!(
            plan(ORG, &rows, next_seq, &before, &after).unwrap_err(),
            format!("Duplicate id {} in budgetLines.", before.budget_lines[0].id)
        );
    }

    #[test]
    fn books_of_another_organisation_are_refused() {
        let (rows, next_seq, before) = loaded();
        assert_eq!(
            plan("org-other", &rows, next_seq, &before, &before).unwrap_err(),
            "Settings belong to another organisation."
        );
    }

    #[test]
    fn an_import_numbers_every_row_on_from_next_seq() {
        let state = demo_state();
        let imported = import(ORG, &serde_json::to_string(&state).unwrap(), 40).unwrap();
        let back = assemble_state(
            &imported.settings_json,
            &imported.counters_json,
            &imported.rows,
        )
        .unwrap();
        // 306 records in the demo books (rust/fixtures/README.md, demo-books.json).
        assert_eq!(
            (back, imported.rows.entries[0].seq, imported.next_seq),
            (state, 40, 40 + 306)
        );
    }

    #[test]
    fn an_import_that_is_not_a_ledger_state_is_refused() {
        assert!(
            import(ORG, r#"{"settings":{}}"#, 0)
                .err()
                .unwrap()
                .starts_with("The books are not a valid ledger state: ")
        );
    }

    #[test]
    fn an_import_of_another_organisations_books_is_refused() {
        let state = serde_json::to_string(&demo_state()).unwrap();
        assert_eq!(
            import("org-other", &state, 0).err().unwrap(),
            "Settings belong to another organisation."
        );
    }
}
