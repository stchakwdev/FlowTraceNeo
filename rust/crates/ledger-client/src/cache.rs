//! The client's own row cache and the state rebuild (`readState`,
//! `flowtrace/src/lib/ledger/spacetime-backend.ts:142-157`).
//!
//! The SDK's row callbacks only send [`RowChange`]s; the connection thread applies a whole
//! message's worth to a [`RowCache`] at once and then announces the organisations it touched.
//! Ledger rows are kept as `ledger_module::tables` row structs, so the rebuild is the module's
//! own [`assemble_state`] and the client and the module can never read rows differently.

use std::collections::{BTreeMap, BTreeSet};

use ledger_core::types::LedgerState;
use ledger_module::codec::{OrgRows, assemble_state};
use ledger_module::tables::{
    AccountRow, AuditEventRow, BankAccountRow, BillRow, BudgetLineRow, BudgetRow, DimensionRow,
    DocRow, ExpenseClaimRow, FiscalPeriodRow, FundRow, GrantRow, JournalEntryRow, JournalLineRow,
    PaymentRow, StatementLineRow, VendorRow,
};

use crate::error::ClientError;
use crate::status::{MemberInfo, Organisation};

/// The fourteen doc tables (`DOC_COLLECTIONS`, `flowtrace/src/lib/ledger/spacetime-codec.ts:21-36`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DocTable {
    /// `account`.
    Account,
    /// `fund`.
    Fund,
    /// `grant`.
    Grant,
    /// `dimension`.
    Dimension,
    /// `fiscal_period`.
    FiscalPeriod,
    /// `budget`.
    Budget,
    /// `budget_line`.
    BudgetLine,
    /// `vendor`.
    Vendor,
    /// `bill`.
    Bill,
    /// `payment`.
    Payment,
    /// `expense_claim`.
    ExpenseClaim,
    /// `bank_account`.
    BankAccount,
    /// `statement_line`.
    StatementLine,
    /// `audit_event`.
    AuditEvent,
}

/// The columns every doc table shares (`ledger_module::tables`, `doc_row!`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocColumns {
    /// `{org_id}:{id}`.
    pub key: String,
    /// The organisation.
    pub org_id: String,
    /// The record's id.
    pub id: String,
    /// Position in the state's collection.
    pub seq: u64,
    /// The record as JSON.
    pub data: String,
}

impl DocColumns {
    /// The columns of a module doc row.
    #[must_use]
    pub fn of<R: DocRow>(row: &R, org_id: &str, id: &str) -> Self {
        Self {
            key: row.key().to_owned(),
            org_id: org_id.to_owned(),
            id: id.to_owned(),
            seq: row.seq(),
            data: row.data().to_owned(),
        }
    }

    fn to_row<R: DocRow>(&self) -> R {
        R::from_columns(
            self.key.clone(),
            self.org_id.clone(),
            self.id.clone(),
            self.seq,
            self.data.clone(),
        )
    }
}

/// The `ledger_org` columns the rebuild needs (`index.ts:42-52`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrgHeader {
    /// The organisation.
    pub org_id: String,
    /// `OrgLedgerSettings` as JSON.
    pub settings_json: String,
    /// `LedgerState.counters` as JSON.
    pub counters_json: String,
}

/// A `member` row, identity as hex (`index.ts:54-66`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemberRow {
    /// `{org_id}:{identity hex}`.
    pub key: String,
    /// The organisation.
    pub org_id: String,
    /// The member.
    pub info: MemberInfo,
}

/// A `command_result` row (`index.ts:142-152`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResultRow {
    /// The organisation the command ran on.
    pub org_id: String,
    /// The caller's request id.
    pub request_id: String,
    /// `JSON.stringify(result)`.
    pub result_json: String,
}

/// One row of any subscribed view.
#[derive(Debug, Clone, PartialEq)]
pub enum Row {
    /// `my_ledger_org`.
    Org(OrgHeader),
    /// `my_member`.
    Member(MemberRow),
    /// `my_journal_entry`.
    Entry(JournalEntryRow),
    /// `my_journal_line`.
    Line(JournalLineRow),
    /// One of the fourteen doc views.
    Doc(DocTable, DocColumns),
    /// `my_command_result`.
    Result(ResultRow),
    /// One of the transparency views, cached apart ([`crate::transparency`]).
    Transparency(crate::transparency::TransparencyRow),
}

/// What a row callback sends: a row that is now present (insert, or the new side of an
/// update), or one that is gone.
#[derive(Debug, Clone, PartialEq)]
pub enum RowChange {
    /// Insert or update: replaces any row under the same key (ADR 0002, decision 4).
    Upsert(Row),
    /// Delete: removes the row under its key only if it is still this row, so a delete and an
    /// insert of one key in the same transaction leave the new row whatever their order.
    Delete(Row),
}

/// One organisation's rows.
#[derive(Debug, Default)]
struct OrgCache {
    header: Option<OrgHeader>,
    members: BTreeMap<String, MemberRow>,
    entries: BTreeMap<String, JournalEntryRow>,
    lines: BTreeMap<String, JournalLineRow>,
    docs: BTreeMap<DocTable, BTreeMap<String, DocColumns>>,
}

impl OrgCache {
    fn is_empty(&self) -> bool {
        self.header.is_none()
            && self.members.is_empty()
            && self.entries.is_empty()
            && self.lines.is_empty()
            && self.docs.values().all(BTreeMap::is_empty)
    }

    fn docs<R: DocRow>(&self, table: DocTable) -> Vec<R> {
        self.docs
            .get(&table)
            .map(|rows| rows.values().map(DocColumns::to_row).collect())
            .unwrap_or_default()
    }
}

/// Puts `row` under `key`, or removes the row under `key` if it equals `row`.
fn put<T: PartialEq>(map: &mut BTreeMap<String, T>, key: &str, row: T, upsert: bool) {
    if upsert {
        map.insert(key.to_owned(), row);
    } else if map.get(key) == Some(&row) {
        map.remove(key);
    }
}

/// Every ledger row of the caller's organisations, by organisation.
#[derive(Debug, Default)]
pub struct RowCache {
    orgs: BTreeMap<String, OrgCache>,
}

impl RowCache {
    /// Applies one change and returns the organisation it touched; `None` for a command result,
    /// which belongs to no books, and for a transparency row, which the client caches apart.
    pub fn apply(&mut self, change: RowChange) -> Option<String> {
        let (row, upsert) = match change {
            RowChange::Upsert(row) => (row, true),
            RowChange::Delete(row) => (row, false),
        };
        let org_id = match &row {
            Row::Org(r) => r.org_id.clone(),
            Row::Member(r) => r.org_id.clone(),
            Row::Entry(r) => r.org_id.clone(),
            Row::Line(r) => r.org_id.clone(),
            Row::Doc(_, r) => r.org_id.clone(),
            Row::Result(_) | Row::Transparency(_) => return None,
        };
        let org = self.orgs.entry(org_id.clone()).or_default();
        match row {
            Row::Org(r) => {
                if upsert {
                    org.header = Some(r);
                } else if org.header.as_ref() == Some(&r) {
                    org.header = None;
                }
            }
            Row::Member(r) => put(&mut org.members, &r.key.clone(), r, upsert),
            Row::Entry(r) => put(&mut org.entries, &r.key.clone(), r, upsert),
            Row::Line(r) => put(&mut org.lines, &r.key.clone(), r, upsert),
            Row::Doc(table, r) => {
                put(
                    org.docs.entry(table).or_default(),
                    &r.key.clone(),
                    r,
                    upsert,
                );
            }
            Row::Result(_) | Row::Transparency(_) => {}
        }
        if org.is_empty() {
            self.orgs.remove(&org_id);
        }
        Some(org_id)
    }

    /// Applies a batch and returns every organisation it touched.
    pub fn apply_all(&mut self, changes: impl IntoIterator<Item = RowChange>) -> BTreeSet<String> {
        changes.into_iter().filter_map(|c| self.apply(c)).collect()
    }

    /// The organisations with books in the cache, in id order. One whose settings do not parse
    /// is left out here; [`RowCache::state`] reports why.
    #[must_use]
    pub fn organisations(&self) -> Vec<Organisation> {
        self.orgs
            .iter()
            .filter_map(|(org_id, org)| {
                let settings = serde_json::from_str(&org.header.as_ref()?.settings_json).ok()?;
                Some(Organisation {
                    org_id: org_id.clone(),
                    settings,
                    members: org.members.values().map(|m| m.info.clone()).collect(),
                })
            })
            .collect()
    }

    /// One organisation's books from its rows (`readState`, `spacetime-backend.ts:142-157`),
    /// through the module's [`assemble_state`].
    ///
    /// # Errors
    ///
    /// [`ClientError::NoLedger`] when there is no `ledger_org` row for `org_id`, and
    /// [`ClientError::Codec`] when a row does not convert.
    pub fn state(&self, org_id: &str) -> Result<LedgerState, ClientError> {
        let org = self
            .orgs
            .get(org_id)
            .ok_or_else(|| ClientError::NoLedger(org_id.to_owned()))?;
        let header = org
            .header
            .as_ref()
            .ok_or_else(|| ClientError::NoLedger(org_id.to_owned()))?;
        let rows = OrgRows {
            entries: org.entries.values().cloned().collect(),
            lines: org.lines.values().cloned().collect(),
            accounts: org.docs::<AccountRow>(DocTable::Account),
            funds: org.docs::<FundRow>(DocTable::Fund),
            grants: org.docs::<GrantRow>(DocTable::Grant),
            dimensions: org.docs::<DimensionRow>(DocTable::Dimension),
            periods: org.docs::<FiscalPeriodRow>(DocTable::FiscalPeriod),
            budgets: org.docs::<BudgetRow>(DocTable::Budget),
            budget_lines: org.docs::<BudgetLineRow>(DocTable::BudgetLine),
            vendors: org.docs::<VendorRow>(DocTable::Vendor),
            bills: org.docs::<BillRow>(DocTable::Bill),
            payments: org.docs::<PaymentRow>(DocTable::Payment),
            claims: org.docs::<ExpenseClaimRow>(DocTable::ExpenseClaim),
            bank_accounts: org.docs::<BankAccountRow>(DocTable::BankAccount),
            statement_lines: org.docs::<StatementLineRow>(DocTable::StatementLine),
            audit: org.docs::<AuditEventRow>(DocTable::AuditEvent),
        };
        Ok(assemble_state(
            &header.settings_json,
            &header.counters_json,
            &rows,
        )?)
    }
}

/// Every row of `rows` and the organisation header, as upserts: what a fresh subscription to an
/// organisation's books delivers. Used by the tests.
#[must_use]
pub fn org_rows_as_changes(header: OrgHeader, rows: OrgRows) -> Vec<RowChange> {
    fn docs<R: DocRow>(table: DocTable, rows: Vec<R>, org_id: &str) -> Vec<RowChange> {
        rows.into_iter()
            .map(|r| {
                // The id is the key after `{org_id}:` (`row_key`, `codec.rs`).
                let id = r
                    .key()
                    .strip_prefix(org_id)
                    .and_then(|k| k.strip_prefix(':'))
                    .unwrap_or_default()
                    .to_owned();
                RowChange::Upsert(Row::Doc(table, DocColumns::of(&r, org_id, &id)))
            })
            .collect()
    }
    let org_id = header.org_id.clone();
    let OrgRows {
        entries,
        lines,
        accounts,
        funds,
        grants,
        dimensions,
        periods,
        budgets,
        budget_lines,
        vendors,
        bills,
        payments,
        claims,
        bank_accounts,
        statement_lines,
        audit,
    } = rows;
    let mut out = vec![RowChange::Upsert(Row::Org(header))];
    out.extend(
        entries
            .into_iter()
            .map(|r| RowChange::Upsert(Row::Entry(r))),
    );
    out.extend(lines.into_iter().map(|r| RowChange::Upsert(Row::Line(r))));
    out.extend(docs(DocTable::Account, accounts, &org_id));
    out.extend(docs(DocTable::Fund, funds, &org_id));
    out.extend(docs(DocTable::Grant, grants, &org_id));
    out.extend(docs(DocTable::Dimension, dimensions, &org_id));
    out.extend(docs(DocTable::FiscalPeriod, periods, &org_id));
    out.extend(docs(DocTable::Budget, budgets, &org_id));
    out.extend(docs(DocTable::BudgetLine, budget_lines, &org_id));
    out.extend(docs(DocTable::Vendor, vendors, &org_id));
    out.extend(docs(DocTable::Bill, bills, &org_id));
    out.extend(docs(DocTable::Payment, payments, &org_id));
    out.extend(docs(DocTable::ExpenseClaim, claims, &org_id));
    out.extend(docs(DocTable::BankAccount, bank_accounts, &org_id));
    out.extend(docs(DocTable::StatementLine, statement_lines, &org_id));
    out.extend(docs(DocTable::AuditEvent, audit, &org_id));
    out
}

#[cfg(test)]
mod tests {
    #![expect(clippy::unwrap_used, reason = "tests may unwrap")]

    use super::*;

    use ledger_module::codec::{counters_json, settings_json, state_to_rows};

    use crate::testing::demo_state;

    fn header(org_id: &str, state: &LedgerState) -> OrgHeader {
        OrgHeader {
            org_id: org_id.to_owned(),
            settings_json: settings_json(&state.settings).unwrap(),
            counters_json: counters_json(&state.counters).unwrap(),
        }
    }

    fn demo_changes(org_id: &str, state: &LedgerState) -> Vec<RowChange> {
        let mut next_seq = 0;
        let rows = state_to_rows(org_id, state, &mut next_seq).unwrap();
        org_rows_as_changes(header(org_id, state), rows)
    }

    fn member(org_id: &str, hex: &str, name: &str) -> MemberRow {
        MemberRow {
            key: format!("{org_id}:{hex}"),
            org_id: org_id.to_owned(),
            info: MemberInfo {
                identity: hex.to_owned(),
                user_id: format!("user-{name}"),
                name: name.to_owned(),
                can_post: true,
                can_approve: false,
                can_close: false,
            },
        }
    }

    #[test]
    fn the_demo_books_rebuild_equal_from_their_rows() {
        let state = demo_state();
        let mut cache = RowCache::default();
        let touched = cache.apply_all(demo_changes("demo", &state));
        assert_eq!(touched, BTreeSet::from(["demo".to_owned()]));
        assert_eq!(cache.state("demo").unwrap(), state);
    }

    #[test]
    fn the_rebuild_does_not_depend_on_arrival_order() {
        let state = demo_state();
        let mut changes = demo_changes("demo", &state);
        changes.reverse();
        let mut cache = RowCache::default();
        cache.apply_all(changes);
        assert_eq!(cache.state("demo").unwrap(), state);
    }

    #[test]
    fn two_organisations_rebuild_apart() {
        let state = demo_state();
        let mut other = ledger_core::setup::create_org_ledger(
            "other",
            "UGX",
            2026,
            &mut ledger_core::env::fixed::test(),
        )
        .unwrap();
        other.counters.insert("JE".to_owned(), 3);
        let mut cache = RowCache::default();
        cache.apply_all(demo_changes("demo", &state));
        cache.apply_all(demo_changes("other", &other));
        assert_eq!(
            (cache.state("demo").unwrap(), cache.state("other").unwrap()),
            (state, other)
        );
        let ids: Vec<String> = cache
            .organisations()
            .into_iter()
            .map(|o| o.org_id)
            .collect();
        assert_eq!(ids, ["demo", "other"]);
    }

    #[test]
    fn an_update_replaces_the_row_under_its_key() {
        let state = demo_state();
        let mut changes = demo_changes("demo", &state);
        let Some(RowChange::Upsert(Row::Doc(DocTable::Vendor, vendor))) = changes
            .iter()
            .find(|c| matches!(c, RowChange::Upsert(Row::Doc(DocTable::Vendor, _))))
            .cloned()
        else {
            panic!("no vendor row");
        };
        let mut renamed: ledger_core::types::Vendor = serde_json::from_str(&vendor.data).unwrap();
        renamed.name = "Renamed Supplies Ltd".to_owned();
        changes.push(RowChange::Upsert(Row::Doc(
            DocTable::Vendor,
            DocColumns {
                data: serde_json::to_string(&renamed).unwrap(),
                ..vendor
            },
        )));
        let mut cache = RowCache::default();
        cache.apply_all(changes);
        let back = cache.state("demo").unwrap();
        assert_eq!(back.vendors.len(), state.vendors.len());
        assert!(back.vendors.contains(&renamed));
    }

    #[test]
    fn a_delete_removes_only_the_row_it_names() {
        let state = demo_state();
        let mut cache = RowCache::default();
        cache.apply_all(demo_changes("demo", &state));
        let old = member("demo", "aa", "Ama");
        let new = MemberRow {
            info: MemberInfo {
                can_close: true,
                ..old.info.clone()
            },
            ..old.clone()
        };
        // A member update arrives as insert of the new row and delete of the old, in either order.
        cache.apply_all([
            RowChange::Upsert(Row::Member(old.clone())),
            RowChange::Upsert(Row::Member(new.clone())),
            RowChange::Delete(Row::Member(old)),
        ]);
        assert_eq!(
            cache.organisations()[0].members,
            std::slice::from_ref(&new.info)
        );
        cache.apply(RowChange::Delete(Row::Member(new)));
        assert_eq!(cache.organisations()[0].members, []);
    }

    #[test]
    fn deleting_every_row_forgets_the_organisation() {
        let state = demo_state();
        let mut cache = RowCache::default();
        let changes = demo_changes("demo", &state);
        cache.apply_all(changes.clone());
        cache.apply_all(changes.into_iter().map(|c| match c {
            RowChange::Upsert(r) | RowChange::Delete(r) => RowChange::Delete(r),
        }));
        assert_eq!(cache.organisations(), []);
        assert!(matches!(cache.state("demo"), Err(ClientError::NoLedger(o)) if o == "demo"));
    }

    #[test]
    fn rows_without_their_ledger_org_row_are_no_ledger_yet() {
        let state = demo_state();
        let mut changes = demo_changes("demo", &state);
        changes.remove(0);
        let mut cache = RowCache::default();
        cache.apply_all(changes);
        assert!(matches!(cache.state("demo"), Err(ClientError::NoLedger(_))));
        assert_eq!(cache.organisations(), []);
    }

    #[test]
    fn a_broken_row_is_a_codec_error() {
        let state = demo_state();
        let mut changes = demo_changes("demo", &state);
        if let Some(RowChange::Upsert(Row::Entry(e))) = changes.get_mut(1) {
            e.status = "open".to_owned();
        }
        let mut cache = RowCache::default();
        cache.apply_all(changes);
        assert_eq!(
            cache.state("demo").unwrap_err().to_string(),
            "The books did not load: \"open\" is not a valid entry status."
        );
    }

    #[test]
    fn command_results_touch_no_books() {
        let mut cache = RowCache::default();
        let touched = cache.apply(RowChange::Upsert(Row::Result(ResultRow {
            org_id: "demo".to_owned(),
            request_id: "r-1".to_owned(),
            result_json: r#"{"type":"none"}"#.to_owned(),
        })));
        assert_eq!(touched, None);
        assert_eq!(cache.organisations(), []);
    }
}
